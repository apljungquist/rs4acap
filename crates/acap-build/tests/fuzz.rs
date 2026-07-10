//! Differential fuzzing of the `acap-build` program.
//!
//! Random but plausible application directories are generated, packaged, and checked for
//! **equivalence**: the produced `.eap` must be byte-identical to that of a reference
//! implementation.

use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

use proptest::{
    prelude::*,
    test_runner::{Config, RngAlgorithm, TestError, TestRng, TestRunner},
};
use serde_json::{json, Map, Value};

const TARGET_ARCH: &str = "aarch64";

// TODO: Extend what aspects of the input we vary
#[derive(Clone, Debug)]
struct AppSpec {
    schema_version: &'static str,
    app_name: String,
    version: String,
    friendly_name: Option<String>,
}

impl AppSpec {
    fn manifest(&self) -> Value {
        let mut setup = Map::new();
        setup.insert("appName".into(), json!(self.app_name));
        if let Some(v) = &self.friendly_name {
            setup.insert("friendlyName".into(), json!(v));
        }
        setup.insert("runMode".into(), json!("never"));
        setup.insert("version".into(), json!(self.version));

        json!({
            "schemaVersion": self.schema_version,
            "acapPackageConf": { "setup": Value::Object(setup) },
        })
    }

    fn materialize(&self, dir: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(dir)?;
        fs::write(
            dir.join("manifest.json"),
            serde_json::to_string_pretty(&self.manifest())?,
        )?;
        fs::write(dir.join("LICENSE"), "All rights reserved.\n")?;
        write_exe(dir, &self.app_name, b"#!/bin/sh\nexit 0\n", true)?;
        Ok(())
    }
}

fn write_exe(dir: &Path, rel_path: &str, content: &[u8], executable: bool) -> anyhow::Result<()> {
    let path = dir.join(rel_path);
    fs::write(&path, content)?;
    let mode = if executable { 0o755 } else { 0o644 };
    fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

fn app_spec() -> impl Strategy<Value = AppSpec> {
    (
        // 1.3 and 1.3.0 sit on the boundary at which the architecture field starts being added;
        // 1.7.0 is a control that both implementations handle identically.
        prop_oneof![Just("1.3"), Just("1.3.0"), Just("1.7.0")],
        "[a-z][a-z0-9_]{0,8}".prop_filter(
            "app name must not shadow a reserved directory name",
            |name| !matches!(name.as_str(), "lib" | "html" | "declarations"),
        ),
        (0u8..20, 0u8..20, 0u8..100).prop_map(|(a, b, c)| format!("{a}.{b}.{c}")),
        prop::option::of(prop_oneof![
            "[A-Za-z][A-Za-z0-9 ]{0,10}",
            // Non-ASCII (U+00C5, U+03A9); exercises differences in JSON string escaping
            // between implementations.
            Just("\u{c5}pp \u{3a9}".to_string()),
        ]),
    )
        .prop_map(
            |(schema_version, app_name, version, friendly_name)| AppSpec {
                schema_version,
                app_name,
                version,
                friendly_name,
            },
        )
}

// Building an input with either implementation
// ============================================

#[derive(Clone, Copy)]
enum Implementation {
    /// The binary under test.
    Native,
    /// The `acap-build` on the `PATH`.
    Reference,
}

impl Implementation {
    fn command(self) -> Command {
        match self {
            Self::Native => Command::new(env!("CARGO_BIN_EXE_acap-build")),
            Self::Reference => reference_command(),
        }
    }
}

/// A command that runs the reference `acap-build` from the SDK.
///
/// The SDK is located via `ACAP_SDK_LOCATION` (the same variable the binary under test reads),
/// defaulting to where the reference hardcodes its manifest tools. Rather than sourcing the SDK's
/// environment-setup script -- which refuses to run under the `LD_LIBRARY_PATH` that cargo sets
/// for test subprocesses -- we set only what the reference needs: its host tools on the `PATH` (so
/// both `acap-build` and the `eap-create.sh` it calls resolve), and the sysroot locations that
/// `eap-create.sh` reads to determine the package architecture.
fn reference_command() -> Command {
    let sdk = std::env::var_os("ACAP_SDK_LOCATION").unwrap_or_else(|| "/opt/axis".into());
    let sdk = Path::new(&sdk);
    let native_sysroot = sdk.join("acapsdk/sysroots/x86_64-pokysdk-linux");
    let target_sysroot = sdk.join("acapsdk/sysroots").join(TARGET_ARCH);

    let existing = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(
        std::iter::once(native_sysroot.join("usr/bin")).chain(std::env::split_paths(&existing)),
    )
    .expect("SDK bin directory contains no path separator");

    let mut command = Command::new("acap-build");
    command
        .env("PATH", path)
        .env("OECORE_NATIVE_SYSROOT", &native_sysroot)
        .env("SDKTARGETSYSROOT", &target_sysroot)
        .env("OECORE_TARGET_SYSROOT", &target_sysroot);
    command
}

#[derive(Debug)]
struct Outcome {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    /// File name and content of the produced EAP, if exactly one was produced.
    eap: Option<(String, Vec<u8>)>,
}

/// Package `spec` with the given implementation and return the outcome. Both implementations are
/// invoked with the same arguments and environment.
fn build(spec: &AppSpec, implementation: Implementation) -> anyhow::Result<Outcome> {
    let scratch = tempfile::tempdir()?;
    let app = scratch.path().join("app");
    spec.materialize(&app)?;

    // TODO: Enable schema validation
    let output = implementation
        .command()
        .args(["--build", "no-build", "--disable-manifest-validation", "."])
        .current_dir(&app)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("OECORE_TARGET_ARCH", TARGET_ARCH)
        .env_remove("RUST_LOG")
        // cargo sets LD_LIBRARY_PATH for test subprocesses, which can make the SDK's python3 and
        // the tools it spawns load the wrong libraries; the reference does not need it.
        .env_remove("LD_LIBRARY_PATH")
        .output()?;

    let mut eaps = Vec::new();
    for entry in fs::read_dir(&app)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("eap") {
            eaps.push(path);
        }
    }
    eaps.sort();
    let eap = match eaps.as_slice() {
        [single] => Some((
            single
                .file_name()
                .expect("path from read_dir has a file name")
                .to_string_lossy()
                .into_owned(),
            fs::read(single)?,
        )),
        _ => None,
    };

    Ok(Outcome {
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        eap,
    })
}

fn compare(spec: &AppSpec) -> Result<(), String> {
    let native = build(spec, Implementation::Native)
        .map_err(|e| format!("running the native build failed: {e:#}"))?;
    let Some((native_name, native_bytes)) = &native.eap else {
        return Err(format!(
            "the native build did not produce exactly one EAP; the generator is expected to \
             produce only buildable inputs\n{}",
            describe("native", &native)
        ));
    };

    let reference_outcome = build(spec, Implementation::Reference)
        .map_err(|e| format!("running the reference build failed: {e:#}"))?;
    let Some((reference_name, reference_bytes)) = &reference_outcome.eap else {
        return Err(format!(
            "the reference build did not produce exactly one EAP\n{}\n{}",
            describe("native", &native),
            describe("reference", &reference_outcome)
        ));
    };

    let context = || {
        format!(
            "{}\n{}",
            describe("native", &native),
            describe("reference", &reference_outcome)
        )
    };
    if native_name != reference_name {
        return Err(format!(
            "EAP file names differ: native produced {native_name:?}, reference produced \
             {reference_name:?}\n{}",
            context()
        ));
    }
    if native_bytes != reference_bytes {
        return Err(format!(
            "EAP contents differ for {native_name:?}\n{}\nThe difference is often in the \
             re-serialized manifest.json.\n{}",
            describe_difference(native_bytes, reference_bytes),
            context()
        ));
    }
    Ok(())
}

fn describe(label: &str, outcome: &Outcome) -> String {
    format!(
        "[{label}] exit code: {:?}\n[{label}] stdout:\n{}\n[{label}] stderr:\n{}",
        outcome.exit_code, outcome.stdout, outcome.stderr
    )
}

fn describe_difference(a: &[u8], b: &[u8]) -> String {
    let offset = a
        .iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .unwrap_or_else(|| a.len().min(b.len()));
    format!(
        "sizes: {} vs {} bytes; first differing byte at offset {offset}",
        a.len(),
        b.len()
    )
}

// Comparing against the reference needs the upstream acap-build, which requires the SDK at
// /opt/axis; the regular test environments do not have it, so this runs only where it is set up,
// e.g. the fuzz workflow. Run it explicitly with `cargo test ... -- --ignored`.
#[ignore = "requires the reference acap-build (SDK at /opt/axis)"]
#[test]
fn generated_inputs_match_reference() {
    let cases: u32 = std::env::var("ACAP_BUILD_FUZZ_CASES")
        .ok()
        .map(|v| v.parse().unwrap())
        .unwrap_or(1);
    let seed: u64 = std::env::var("ACAP_BUILD_FUZZ_SEED")
        .ok()
        .map(|v| v.parse().unwrap())
        .unwrap_or(0);
    let mut rng_seed = [0u8; 32];
    for (dst, src) in rng_seed.iter_mut().zip(seed.to_le_bytes()) {
        *dst = src;
    }

    let config = Config {
        cases,
        failure_persistence: None,
        ..Config::default()
    };
    let rng = TestRng::from_seed(RngAlgorithm::ChaCha, &rng_seed);

    let r = TestRunner::new_with_rng(config, rng).run(&app_spec(), |spec| {
        compare(&spec).map_err(TestCaseError::fail)
    });

    match r {
        Ok(()) => {}
        Err(TestError::Fail(reason, spec)) => {
            panic!("Property violated by {spec:#?}:\n{reason}")
        }
        Err(e @ TestError::Abort(_)) => panic!("Fuzzing aborted: {e}"),
    }
}

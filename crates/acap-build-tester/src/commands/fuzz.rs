use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use acap_build::Cli;
use anyhow::{bail, Context};
use clap::ValueEnum;
use proptest::test_runner::{Config, RngAlgorithm, TestCaseError, TestError, TestRng, TestRunner};

use crate::{
    input::{arbitrary_input, Input},
    invocation::{build_with, Environment},
    source::{Manifest, Source},
};

fn check(candidate_exe: &Path, input: &Input) -> anyhow::Result<()> {
    let candidate_dir = tempfile::tempdir()?;
    input.source.materialize_in(candidate_dir.path())?;
    let candidate = build_with(
        candidate_exe,
        candidate_dir.path(),
        input.invocation.clone(),
    )
    .context("building with the candidate")?;

    let reference_dir = tempfile::tempdir()?;
    input.source.materialize_in(reference_dir.path())?;
    let reference = build_with("acap-build", reference_dir.path(), input.invocation.clone())
        .context("building with the reference")?;

    if candidate.essence() != reference.essence() {
        bail!("the candidate succeeded but does not match the reference:\n{candidate:#?}\n{reference:#?}");
    }
    Ok(())
}

fn fuzz(
    candidate: &Path,
    environment: Environment,
    cases: u32,
    seed: u64,
) -> Result<(), Box<TestError<Input>>> {
    let mut rng_seed = [0u8; 32];
    for (dst, src) in rng_seed.iter_mut().zip(seed.to_le_bytes()) {
        *dst = src;
    }

    let config = Config {
        cases,
        failure_persistence: None,
        // Accept a rejection rate of 80%
        // TODO: Consider tuning this value and/or making it configurable
        max_global_rejects: 4 * cases,
        ..Config::default()
    };
    let rng = TestRng::from_seed(RngAlgorithm::ChaCha, &rng_seed);

    let compared = AtomicU64::new(0);
    let rejected = AtomicU64::new(0);

    let result = TestRunner::new_with_rng(config, rng)
        .run(&arbitrary_input(environment), |input| {
            match input.invocation.error_for_conservative() {
                Ok(()) => {
                    compared.fetch_add(1, Ordering::Relaxed);
                    check(candidate, &input).map_err(|e| TestCaseError::fail(format!("{e:#}")))
                }
                Err(e) => {
                    rejected.fetch_add(1, Ordering::Relaxed);
                    Err(TestCaseError::reject(e.to_string()))
                }
            }
        })
        .map_err(Box::new);

    let compared = compared.load(Ordering::Relaxed);
    let rejected = rejected.load(Ordering::Relaxed);
    let total = compared + rejected;
    if total != 0 {
        log::info!(
            "The candidate was compared to the reference on {compared} and rejected {rejected} of {total} inputs ({percent:.1}% rejected).",
            percent = 100.0 * rejected as f64 / total as f64,
        );
    }

    result
}

/// A stable 64-bit FNV-1a hash of `bytes`.
fn content_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The name of a saved example's source directory: a checksum of the source, computed from the
/// in-memory model so distinct sources never collide onto one directory.
///
/// The same checksum prefixes the example's recorded invocation files (see `save_example`), tying
/// them to this directory. Curation may replace that prefix — on the directory and every invocation
/// file — with a meaningful name, or leave it.
fn example_name(input: &Input) -> String {
    // Destructure both structs so that adding a field to `Source` or `Manifest` fails to compile
    // here until someone decides whether it affects the packaged tree and so belongs in `identity`.
    let Source {
        manifest,
        manifest_name,
        additional_files,
        html,
    } = &input.source;
    let Manifest {
        schema_version,
        app_name,
        version,
        friendly_name,
    } = manifest;

    // Every field that affects the packaged tree, joined with separators the fields cannot contain
    // (`\0`, and `,` between file names, which match `[a-z][a-z0-9_]*`). Distinct sources therefore
    // never map to the same checksum and clobber each other. Explicit formatting rather than `Debug`
    // keeps the source-to-name mapping a stable contract now that the name is written to disk.
    let additional_files = additional_files
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(",");
    let identity = format!(
        "{schema_version}\0{app_name}\0{version}\0{}\0{manifest_name}\0{html}\0{additional_files}",
        friendly_name.as_deref().unwrap_or(""),
    );
    format!("{:016x}", content_hash(identity.as_bytes()))
}

/// Materialize a failing input as a replayable example.
///
/// The source is written into `example_dir` and the invocation into a file in the sibling
/// `invocations/<example>` directory that `replay` reads. The file is named
/// `<source-checksum>-<arch>-<invocation-checksum>`: the source-checksum prefix (the example
/// directory's name) ties it to its source so curation can rename both together, while the
/// invocation-checksum keeps invocations of one source that differ — e.g. with vs. without manifest
/// validation — from colliding. One example can therefore hold any number of invocations.
///
/// Design note — harvested vs. curated examples. These records are *harvested* from fuzzing rather
/// than hand-authored, which is a deliberate bet that not every implementation shares: sty1va keeps
/// its replay records curated and checked in by hand. Both are valid. As long as we harvest, treat
/// a saved record as a *seed for a human to curate*, not an auto-grown CI corpus: the invocation is
/// coupled to the host that produced it — its architecture, and the absolute sysroot paths that the
/// `replay` command overrides with the host's — so an unreviewed record is only known to reproduce
/// where it was born. Growing the checked-in corpus straight from CI
/// artifacts would smuggle that host-coupling into the tree; a human deciding what to keep is what
/// keeps the examples portable.
fn save_example(example_dir: &Path, input: &Input) -> anyhow::Result<PathBuf> {
    input.source.materialize_in(example_dir)?;

    let name = example_dir
        .file_name()
        .context("--save-failing path has no final component")?;
    let invocations_dir = example_dir
        .parent()
        .context("--save-failing path has no parent")?
        .with_file_name("invocations")
        .join(name);
    fs::create_dir_all(&invocations_dir)?;

    // Record the whole invocation. `path` is an ephemeral scratch directory that replay overrides
    // with a copy of its own, so store a neutral placeholder rather than this run's build path.
    let invocation = Cli {
        path: PathBuf::from("."),
        ..input.invocation.clone()
    };
    let mut json = serde_json::to_string_pretty(&invocation)?;
    json.push('\n');

    // `<source-checksum>-<arch>-<invocation-checksum>`. The source-checksum prefix is the example
    // directory's own name, tying the file to its source. The architecture follows because it is
    // what a reader most wants to see and what replay gates on; the trailing hash of the whole
    // invocation only disambiguates invocations of one source that vary in validation, epoch,
    // conservative mode, and so on, and keeps the name stable so re-recording overwrites in place.
    let arch = input
        .invocation
        .oecore_target_arch
        .to_possible_value()
        .expect("every architecture variant has a name");
    let stem = format!(
        "{}-{}-{:016x}",
        name.to_string_lossy(),
        arch.get_name(),
        content_hash(json.as_bytes())
    );
    let path = invocations_dir.join(format!("{stem}.json"));
    fs::write(&path, json)?;
    Ok(path)
}

#[derive(clap::Parser)]
pub struct FuzzCommand {
    /// Number of random inputs to try.
    #[clap(long, env = "ACAP_BUILD_FUZZ_CASES", default_value_t = 1)]
    cases: u32,
    /// Seed for the random number generator.
    #[clap(long, env = "ACAP_BUILD_FUZZ_SEED", default_value_t = 0)]
    seed: u64,
    /// Apps directory under which to record the shrunk failing input as an example, if any.
    ///
    /// A subdirectory named after the source is created here for the source, and the invocation is
    /// written to the sibling `invocations/<example>` directory. Point this at a scratch location
    /// outside the committed corpus (see the `fuzz_equivalence` Makefile target): saved examples
    /// are seeds for a human to curate into the `replay` tree, not committed as recorded.
    #[clap(long)]
    save_failing: Option<PathBuf>,
    #[clap(flatten)]
    environment: Environment,
}

impl FuzzCommand {
    pub fn exec(self, candidate: &Path) -> anyhow::Result<()> {
        let Self {
            cases,
            seed,
            save_failing,
            environment,
        } = self;

        match fuzz(candidate, environment, cases, seed).map_err(|e| *e) {
            Ok(()) => Ok(()),
            Err(TestError::Fail(reason, input)) => {
                let saved = match &save_failing {
                    Some(apps_dir) => {
                        let example_dir = apps_dir.join(example_name(&input));
                        let path = save_example(&example_dir, &input)
                            .context("saving the failing input")?;
                        format!("\nSaved failing example to {path:?}")
                    }
                    None => String::new(),
                };
                bail!("Property violated by {input:#?}:\n{reason}{saved}")
            }
            Err(e @ TestError::Abort(_)) => bail!("Fuzzing aborted: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use acap_build::{BuildOption, OpenEmbeddedTargetArchitecture, DEFAULT_ACAP_SDK_LOCATION};
    use rs4a_eap::{AcapBuildImpl, Mtime};

    use super::*;
    use crate::source::DEFAULT_MANIFEST_NAME;

    fn sample_invocation() -> Cli {
        Cli {
            path: PathBuf::from("."),
            build: BuildOption::NoBuild,
            manifest: PathBuf::from("manifest.json"),
            additional_file: vec![PathBuf::from("extra.txt")],
            disable_manifest_validation: true,
            oecore_target_arch: OpenEmbeddedTargetArchitecture::Arm,
            oecore_native_sysroot: Some(PathBuf::from("/native")),
            sdk_target_sysroot: Some(PathBuf::from("/target/armv7hf")),
            acap_sdk_location: PathBuf::from("/opt/axis/"),
            source_date_epoch: Some(Mtime::try_from(0).unwrap()),
            acap_build_impl: AcapBuildImpl::Equivalent,
            conservative: false,
        }
    }

    #[test]
    fn recorded_invocation_round_trips() {
        let cli = sample_invocation();

        let json = serde_json::to_string_pretty(&cli).unwrap();
        // Non-default fields are written with the on-disk spellings the recorded examples depend on.
        assert!(json.contains("\"no-build\""), "{json}");
        assert!(json.contains("\"arm\""), "{json}");
        // A field not read from the environment is written even at its default.
        assert!(json.contains("manifest.json"), "{json}");
        // Environment-read fields left at their default are omitted (and restored on read).
        assert!(!json.contains("acap_sdk_location"), "{json}");
        assert!(!json.contains("acap_build_impl"), "{json}");
        assert!(!json.contains("conservative"), "{json}");

        let back: Cli = serde_json::from_str(&json).unwrap();
        assert_eq!(cli, back);

        // A non-default implementation is still written, with its spelling locked.
        let compatible = Cli {
            acap_build_impl: AcapBuildImpl::Compatible,
            ..sample_invocation()
        };
        let json = serde_json::to_string_pretty(&compatible).unwrap();
        assert!(json.contains("\"compatible\""), "{json}");
        assert_eq!(serde_json::from_str::<Cli>(&json).unwrap(), compatible);
    }

    #[test]
    fn omitted_environment_fields_are_restored_to_clap_defaults() {
        // A record always carries the fields that are not read from the environment; the
        // environment-read fields may be omitted, and deserialization restores each to the default
        // clap would have assigned.
        let json = r#"{
            "path": ".",
            "build": "no-build",
            "manifest": "manifest.json",
            "additional_file": [],
            "disable_manifest_validation": false,
            "oecore_target_arch": "arm"
        }"#;
        let cli: Cli = serde_json::from_str(json).unwrap();

        assert_eq!(cli.oecore_native_sysroot, None);
        assert_eq!(cli.sdk_target_sysroot, None);
        assert_eq!(
            cli.acap_sdk_location,
            PathBuf::from(DEFAULT_ACAP_SDK_LOCATION)
        );
        assert_eq!(cli.source_date_epoch, None);
        assert_eq!(cli.acap_build_impl, AcapBuildImpl::Equivalent);
        assert!(!cli.conservative);
    }

    #[test]
    fn environment_default_fields_are_omitted_on_serialize() {
        // Environment-read fields at their clap default (or `None`) are dropped; every other field
        // is written even at its default. Adding a `Cli` field forces this literal to be updated,
        // and a mis-wired serde attr surfaces as an unexpected key in the JSON below.
        let cli = Cli {
            path: PathBuf::from("."),
            oecore_target_arch: OpenEmbeddedTargetArchitecture::Aarch64,
            // Not read from the environment: written even when left at the default.
            build: BuildOption::Make,
            manifest: PathBuf::from("manifest.json"),
            additional_file: Vec::new(),
            disable_manifest_validation: false,
            // Read from the environment and at their default: dropped.
            oecore_native_sysroot: None,
            sdk_target_sysroot: None,
            acap_sdk_location: PathBuf::from(DEFAULT_ACAP_SDK_LOCATION),
            source_date_epoch: None,
            acap_build_impl: AcapBuildImpl::Equivalent,
            conservative: false,
        };

        let json: serde_json::Value = serde_json::to_value(&cli).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "path": ".",
                "build": "make",
                "manifest": "manifest.json",
                "additional_file": [],
                "disable_manifest_validation": false,
                "oecore_target_arch": "aarch64",
            }),
        );
    }

    fn sample_source() -> Source {
        Source {
            manifest: Manifest {
                schema_version: "1.3",
                app_name: "myapp".to_string(),
                version: "1.0.0".to_string(),
                friendly_name: None,
            },
            manifest_name: DEFAULT_MANIFEST_NAME.to_string(),
            additional_files: BTreeSet::new(),
            html: false,
        }
    }

    #[test]
    fn save_example_writes_replayable_files() {
        let tmp = tempfile::tempdir().unwrap();
        let example_dir = tmp.path().join("data").join("myapp");
        let input = Input {
            source: sample_source(),
            invocation: sample_invocation(),
        };

        let path = save_example(&example_dir, &input).unwrap();

        // The source lands in the example dir; the invocation in the sibling `invocations` tree,
        // named `<example>-<arch>-<hash>` (the example dir name prefixes it), where `replay` looks.
        assert!(example_dir.join("manifest.json").exists());
        let invocations_dir = tmp.path().join("invocations").join("myapp");
        assert_eq!(path.parent().unwrap(), invocations_dir);
        assert!(
            path.file_stem()
                .unwrap()
                .to_string_lossy()
                .starts_with("myapp-arm-"),
            "{path:?}"
        );

        // The recorded invocation deserializes back with its `path` relative to the example dir.
        let recorded: Cli = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(recorded, input.invocation);

        // Re-recording the same invocation is idempotent: it overwrites its own file.
        assert_eq!(save_example(&example_dir, &input).unwrap(), path);
    }

    #[test]
    fn example_name_is_the_source_checksum() {
        let base = Input {
            source: sample_source(),
            invocation: sample_invocation(),
        };
        // The name depends only on the source, so invocations of one source (here differing in
        // conservative mode) share a directory and accumulate together.
        let other_invocation = Input {
            source: sample_source(),
            invocation: Cli {
                conservative: true,
                ..sample_invocation()
            },
        };
        assert_eq!(example_name(&base), example_name(&other_invocation));

        // Just a 16-hex-digit checksum; a human prefix is added only during curation.
        let name = example_name(&base);
        assert_eq!(name.len(), 16, "{name}");
        assert!(name.bytes().all(|b| b.is_ascii_hexdigit()), "{name}");

        // A different source lands in a different directory rather than clobbering the first.
        let other_source = Input {
            source: Source {
                html: !sample_source().html,
                ..sample_source()
            },
            invocation: sample_invocation(),
        };
        assert_ne!(example_name(&base), example_name(&other_source));
    }

    #[test]
    fn one_example_can_hold_several_invocations() {
        let tmp = tempfile::tempdir().unwrap();
        let example_dir = tmp.path().join("data").join("myapp");

        // Two invocations of the same example that differ only in conservative mode; nothing about
        // the architecture distinguishes them.
        let base = sample_invocation();
        let lenient = Input {
            source: sample_source(),
            invocation: Cli {
                conservative: false,
                ..base.clone()
            },
        };
        let conservative = Input {
            source: sample_source(),
            invocation: Cli {
                conservative: true,
                ..base
            },
        };

        let lenient_path = save_example(&example_dir, &lenient).unwrap();
        let conservative_path = save_example(&example_dir, &conservative).unwrap();

        // Both share the source-checksum prefix but the trailing invocation checksum differs, so
        // they are stored side by side rather than one overwriting the other.
        assert_ne!(lenient_path, conservative_path);
        assert!(lenient_path.exists());
        assert!(conservative_path.exists());
    }
}

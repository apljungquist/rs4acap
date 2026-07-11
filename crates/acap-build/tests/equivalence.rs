use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::Context;
use expect_test::expect;

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&from, &to);
        } else {
            // `fs::copy` preserves the mode, which the executable and scripts rely on.
            fs::copy(&from, &to).unwrap();
        }
    }
}

/// Build the app in `tests/data/<name>` and return the exit code of the build together with a
/// checksum of the produced EAP.
fn build_and_checksum(name: &str, extra_args: &[&str]) -> (Option<i32>, String) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture = Path::new(manifest_dir).join("tests/data").join(name);

    // Build in a scratch copy so the generated files don't pollute the fixture.
    let scratch = tempfile::tempdir().unwrap();
    let app = scratch.path().join(name);
    copy_dir(&fixture, &app);

    let output = Command::new(env!("CARGO_BIN_EXE_acap-build"))
        .args(["--build", "no-build"])
        .args(extra_args)
        .arg(&app)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("OECORE_TARGET_ARCH", "aarch64")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();

    let eap_file_path = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
    assert!(
        output.status.success(),
        "acap-build exited with {}: {eap_file_path:?}",
        output.status
    );

    // TODO: Use a stable hashing algorithm; DefaultHasher is not guaranteed
    let contents = fs::read(&eap_file_path)
        .context(format!("{eap_file_path:?}"))
        .unwrap();
    let mut hasher = DefaultHasher::new();
    contents.hash(&mut hasher);
    (output.status.code(), format!("{:016x}", hasher.finish()))
}

// TODO: Add facilities for comparing with upstream automatically
// TODO: Make `acap-build` compatible with all supported dev environments of this propject
#[ignore = "requires a tier 2 developer environment"]
#[test]
fn example_app_output_matches_snapshot() {
    // TODO: Also assert other properties of the output, such as name and location of artefacts
    expect![[r#"
        (
            Some(
                0,
            ),
            "ee74935e2c809417",
        )
    "#]]
    .assert_debug_eq(&build_and_checksum("example_app", &[]));
}

#[ignore = "requires a tier 2 developer environment"]
#[test]
fn schema_1_3_app_output_matches_snapshot() {
    // Validation is disabled because that is how the example was built when it was found.
    // TODO: Look into whether it can be enabled to stay close to the idealized model
    expect![[r#"
        (
            Some(
                0,
            ),
            "246baeeded73541b",
        )
    "#]]
    .assert_debug_eq(&build_and_checksum(
        "schema_1_3_app",
        &["--disable-manifest-validation"],
    ));
}

#[ignore = "requires a tier 2 developer environment"]
#[test]
fn non_ascii_app_output_matches_snapshot() {
    // Validation is disabled because that is how the example was built when it was found.
    // TODO: Look into whether it can be enabled to stay close to the idealized model
    expect![[r#"
        (
            Some(
                0,
            ),
            "2306cff73c24e010",
        )
    "#]]
    .assert_debug_eq(&build_and_checksum(
        "non_ascii_app",
        &["--disable-manifest-validation"],
    ));
}

#[ignore = "requires a tier 2 developer environment"]
#[test]
fn empty_http_config_app_output_matches_snapshot() {
    // Validation is disabled because that is how the example was built when it was found.
    // TODO: Look into whether it can be enabled to stay close to the idealized model
    expect![[r#"
        (
            Some(
                0,
            ),
            "7080805ecf289485",
        )
    "#]]
    .assert_debug_eq(&build_and_checksum(
        "empty_http_config_app",
        &["--disable-manifest-validation"],
    ));
}

#[ignore = "requires a tier 2 developer environment"]
#[test]
fn directory_http_config_app_output_matches_snapshot() {
    // Validation is disabled because that is how the example was built when it was found.
    // TODO: Look into whether it can be enabled to stay close to the idealized model
    expect![[r#"
        (
            Some(
                0,
            ),
            "e8d8187330c90010",
        )
    "#]]
    .assert_debug_eq(&build_and_checksum(
        "directory_http_config_app",
        &["--disable-manifest-validation"],
    ));
}

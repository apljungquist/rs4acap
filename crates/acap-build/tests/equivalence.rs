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

// TODO: Add facilities for comparing with upstream automatically
// TODO: Make `acap-build` compatible with all supported dev environments of this propject
#[ignore = "does not work on macOS"]
#[test]
fn example_app_output_matches_snapshot() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture = Path::new(manifest_dir).join("tests/data/example_app");

    // Build in a scratch copy so the generated files don't pollute the fixture.
    let scratch = tempfile::tempdir().unwrap();
    let app = scratch.path().join("example_app");
    copy_dir(&fixture, &app);

    let output = Command::new(env!("CARGO_BIN_EXE_acap-build"))
        .args(["--build", "no-build"])
        .arg(&app)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("OECORE_TARGET_ARCH", "aarch64")
        .stderr(Stdio::inherit())
        .output()
        .unwrap();

    let eap_file_path = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
    assert!(
        output.status.success(),
        "acap-built existed with {}: {eap_file_path:?}",
        output.status
    );

    // TODO: Use a stable hashing algorithm; DefaultHasher is not guaranteed
    let contents = fs::read(&eap_file_path)
        .context(format!("{eap_file_path:?}"))
        .unwrap();
    let mut hasher = DefaultHasher::new();
    contents.hash(&mut hasher);
    let checksum = format!("{:016x}", hasher.finish());

    // TODO: Also assert other properties of the output, such as name and location of artefacts
    expect![[r#"
        (
            Some(
                0,
            ),
            "ee74935e2c809417",
        )
    "#]]
    .assert_debug_eq(&(output.status.code(), checksum));
}

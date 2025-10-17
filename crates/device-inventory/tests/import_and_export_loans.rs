use std::{
    collections::HashMap,
    fs,
    io::BufRead,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::SystemTime,
};

fn data_dir(now: SystemTime) -> PathBuf {
    format!(
        "{}/{}",
        env!("CARGO_TARGET_TMPDIR"),
        now.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    )
    .into()
}

fn device_inventory_command(data_dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_device-inventory"));
    cmd.args(["--inventory", data_dir.as_os_str().to_str().unwrap()])
        .arg("--offline")
        .env("RUST_LOG", "debug");
    cmd
}

// TODO: Consider replacing with a generated files pattern
#[test]
fn can_export_loans_from_get_response() {
    let now = SystemTime::now();
    let data_dir = data_dir(now);

    fs::create_dir_all(&data_dir).unwrap();
    fs::copy("test-data/devices.json", data_dir.join("devices.json")).unwrap();

    let output = device_inventory_command(&data_dir)
        .arg("activate")
        .args(["--destination", "environment"])
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    let mut exports = HashMap::new();
    for line in output.stdout.lines() {
        let line = line.unwrap();
        let (k, v) = line
            .strip_prefix("export ")
            .unwrap()
            .split_once('=')
            .unwrap();
        exports.insert(k.to_string(), v.to_string());
    }
    assert_eq!(exports["AXIS_DEVICE_IP"], "195.60.68.14");
    assert_eq!(exports["AXIS_DEVICE_USER"], "VLTuser");
    assert_eq!(exports["AXIS_DEVICE_PASS"], "nYy3cuvX");
    assert_eq!(exports["AXIS_DEVICE_SSH_PORT"], "22051");
    assert_eq!(exports["AXIS_DEVICE_HTTP_PORT"], "12051");
    assert_eq!(exports["AXIS_DEVICE_HTTPS_PORT"], "42051");
    assert_eq!(exports["AXIS_DEVICE_HTTPS_SELF_SIGNED"], "1");
}

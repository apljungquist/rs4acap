use std::{
    collections::HashMap,
    fs,
    io::{BufRead, Write},
    process::{Command, Stdio},
    time::SystemTime,
};

fn device_inventory_command(now: SystemTime) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_device-inventory"));
    cmd.arg(format!(
        "--inventory={}/{}",
        env!("CARGO_TARGET_TMPDIR"),
        now.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    ))
    .arg("--offline")
    .env("RUST_LOG", "debug");
    cmd
}

// TODO: Consider replacing with a generated files pattern
#[test]
fn can_export_loans_from_get_response() {
    let now = SystemTime::now();

    let json_str = fs::read_to_string("test-data/get-loans-response.json").unwrap();
    let mut child = device_inventory_command(now)
        .arg("import")
        .arg("--source=json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(json_str.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"");

    let output = device_inventory_command(now)
        .arg("export")
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

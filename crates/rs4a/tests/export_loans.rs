use std::{
    collections::HashMap,
    fs,
    io::{BufRead, Write},
    process::Command,
};

const CARGO_BIN_EXE: &str = env!("CARGO_BIN_EXE_rs4a");

#[test]
fn can_export_loans_from_get_response() {
    let json_str = fs::read_to_string("tests/export_loans/get-loans-response.json").unwrap();
    let mut child = Command::new(CARGO_BIN_EXE)
        .arg("export-loans")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
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
    assert_eq!(exports["AXIS_DEVICE_IP"], "195.60.68.51");
    assert_eq!(exports["AXIS_DEVICE_USER"], "VLTuser");
    assert_eq!(exports["AXIS_DEVICE_PASS"], "nYy3cuvX");
    assert_eq!(exports["AXIS_DEVICE_HTTP_PORT"], "12051");
    assert_eq!(exports["AXIS_DEVICE_HTTPS_PORT"], "42051");
    assert_eq!(exports["AXIS_DEVICE_SSH_PORT"], "22051");
}

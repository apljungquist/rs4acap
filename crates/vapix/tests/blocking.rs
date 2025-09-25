use std::time::Duration;

use rs4a_vapix::{
    apis, authorization_headers, client_blocking::BlockingClient,
    json_rpc_http_blocking::BlockingJsonRpcHttp, Scheme,
};

fn test_client() -> Option<BlockingClient> {
    let Some(device) = rs4a_dut::Device::from_anywhere().unwrap() else {
        eprintln!("No device configured, skipping test.");
        return None;
    };
    let rs4a_dut::Device {
        host,
        username,
        password,
        http_port: _,
        https_port,
        ssh_port: _,
    } = device;
    Some(BlockingClient {
        scheme: Scheme::Secure,
        host,
        port: https_port,
        client: reqwest::blocking::Client::builder()
            .default_headers(authorization_headers(&username, &password))
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap(),
    })
}

// Compare these with their namesake from `smoke_tests.rs`.

#[test]
fn basic_device_info_get_all_unrestricted_properties_returns_ok() {
    let Some(client) = test_client() else {
        return;
    };
    apis::basic_device_info_1::get_all_unrestricted_properties()
        .send_with_timeout(&client, Duration::from_secs(5))
        .unwrap();
}

#[test]
fn system_ready_system_ready_returns_ok() {
    let Some(client) = test_client() else {
        return;
    };
    apis::system_ready_1::system_ready()
        .send_with_timeout(&client, Duration::from_secs(5))
        .unwrap();
}

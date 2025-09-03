use std::env;

use rs4a_vapix::{Client, ClientBuilder};

async fn test_client() -> Option<Client> {
    if env::var_os("AXIS_DEVICE_IP").is_some() {
        Some(
            ClientBuilder::from_env()
                .unwrap()
                .with_inner(|b| b.danger_accept_invalid_certs(true))
                .build_with_automatic_scheme()
                .await
                .unwrap(),
        )
    } else {
        eprintln!("No device configured, skipping test.");
        None
    }
}

#[tokio::test]
async fn basic_device_info_get_all_unrestricted_properties_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };
    client
        .basic_device_info_1()
        .get_all_unrestricted_properties()
        .send()
        .await
        .unwrap();
}

#[tokio::test]
async fn jpg_get_image_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };
    client
        .jpg_3()
        .get_image()
        .compression(100)
        .send()
        .await
        .unwrap();
}

#[tokio::test]
async fn system_ready_system_ready_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };
    client.system_ready_1().system_ready().send().await.unwrap();
}

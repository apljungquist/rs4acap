use std::{ops::Rem, time::SystemTime};

use log::LevelFilter;
use reqwest::Method;
use rs4a_vapix::{
    action1::Condition,
    apis,
    json_rpc_http::{JsonRpcHttp, JsonRpcHttpLossless},
    recording_group_2::{
        ContainerFormat, ContentEncryption, Encryption, KeyEncryption, ProtectionScheme, PublicKey,
    },
    rest_http::RestHttp,
    soap_http::SoapHttpRequest,
    Client, ClientBuilder,
};
use serde_json::json;

async fn test_client() -> Option<Client> {
    let _ = env_logger::Builder::new()
        .filter_level(LevelFilter::Trace)
        .parse_default_env()
        .is_test(true)
        .try_init();

    let Some(client) = ClientBuilder::from_dut().unwrap() else {
        eprintln!("No device configured, skipping test.");
        return None;
    };
    Some(
        client
            .with_inner(|b| b.danger_accept_invalid_certs(true))
            .build_with_automatic_scheme()
            .await
            .unwrap(),
    )
}

async fn delete_destination(client: &Client, id: &str) {
    client
        .request(
            Method::DELETE,
            &format!("config/rest/remote-object-storage/v1beta/destinations/{id}"),
        )
        .unwrap()
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

fn somewhat_unique_name(prefix: &str) -> String {
    let four_weeks_as_seconds = 4 * 7 * 24 * 60 * 60;
    let suffix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .rem(four_weeks_as_seconds);
    format!("{prefix}{suffix}")
}

#[tokio::test]
async fn action_1_get_action_configurations_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };
    apis::action_1::get_action_configurations()
        .send(&client)
        .await
        .unwrap();
}

#[tokio::test]
async fn action_1_add_and_get_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };

    let action_configuration_id =
        apis::action_1::add_action_configuration("com.axis.action.fixed.ledcontrol")
            .param("led", "statusled")
            .param("color", "green,none")
            .param("duration", "1")
            .param("interval", "250")
            .send(&client)
            .await
            .unwrap()
            .configuration_id;

    let action_rule_name = "smoke test rule";
    let action_rule_id =
        apis::action_1::add_action_rule(action_rule_name.to_string(), action_configuration_id)
            .condition(Condition {
                topic_expression: "tns1:Device/tnsaxis:Status/SystemReady".to_string(),
                message_content: r#"boolean(//SimpleItem[@Name="ready" and @Value="1"])"#
                    .to_string(),
            })
            .send(&client)
            .await
            .unwrap()
            .id;

    let actions_rules = apis::action_1::get_action_rules()
        .send(&client)
        .await
        .unwrap()
        .action_rules
        .action_rule;

    let action_rule = actions_rules
        .into_iter()
        .find(|r| r.rule_id == action_rule_id)
        .unwrap();

    assert_eq!(action_rule.name, action_rule_name);
}

#[tokio::test]
async fn basic_device_info_get_all_properties_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };
    apis::basic_device_info_1::get_all_properties()
        .send_lossless(&client)
        .await
        .unwrap();
}

#[tokio::test]
async fn basic_device_info_get_all_unrestricted_properties_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };
    apis::basic_device_info_1::get_all_unrestricted_properties()
        .send_lossless(&client)
        .await
        .unwrap();
}

#[tokio::test]
async fn event_1_get_event_instances_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };
    apis::event_1::get_event_instances()
        .send(&client)
        .await
        .unwrap();
}

#[tokio::test]
async fn jpg_get_image_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };
    apis::jpg_3::get_image()
        .compression(100)
        .send(&client)
        .await
        .unwrap();
}

#[tokio::test]
async fn recording_group_2_crud() {
    let Some(client) = test_client().await else {
        return;
    };

    let expected_recording_destination_id =
        somewhat_unique_name("smoke_test_recording_destination_");

    let actual_destination_id = apis::remote_object_storage_1::create_destinations()
        .data(json!({
            "id": expected_recording_destination_id,
            "s3": {
                "accessKeyId": "myAccessKeyId",
                "secretAccessKey": "mySecretAccessKey",
                "bucket": "myBucket",
                "url": "https://s3.eu-north-1.amazonaws.com",
            }
        }))
        .send(&client)
        .await
        .unwrap()
        .id;

    assert_eq!(expected_recording_destination_id, actual_destination_id);

    let created = apis::recording_group_2::create(&expected_recording_destination_id)
        .send(&client)
        .await
        .unwrap();

    let fetched = apis::recording_group_2::get(created.id.clone())
        .send(&client)
        .await
        .unwrap();
    assert_eq!(fetched.id, created.id);

    let all = apis::recording_group_2::list().send(&client).await.unwrap();
    assert!(all.iter().any(|g| g.id == created.id));

    apis::recording_group_2::delete(created.id.clone())
        .send(&client)
        .await
        .unwrap();

    let all = apis::recording_group_2::list().send(&client).await.unwrap();
    assert!(!all.iter().any(|g| g.id == created.id));

    delete_destination(&client, &expected_recording_destination_id).await;
}

// TODO: Move tests that are not nullipotent out of smoke tests
#[tokio::test]
async fn ssh_1_crud_user_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };

    let username = somewhat_unique_name("smoke_test_ssh_user_");
    apis::ssh_1::add_user(&username, " ")
        .comment("First comment")
        .send(&client)
        .await
        .unwrap();
    apis::ssh_1::set_user(username)
        .comment("")
        .send(&client)
        .await
        .unwrap();
}

#[tokio::test]
async fn recording_group_2_crud_with_content_encryption() {
    let Some(client) = test_client().await else {
        return;
    };

    let dest_id = somewhat_unique_name("smoke_test_recording_destination_");

    apis::remote_object_storage_1::create_destinations()
        .data(json!({
            "id": dest_id,
            "s3": {
                "accessKeyId": "myAccessKeyId",
                "secretAccessKey": "mySecretAccessKey",
                "bucket": "myBucket",
                "url": "https://s3.eu-north-1.amazonaws.com",
            }
        }))
        .send(&client)
        .await
        .unwrap();

    let created = apis::recording_group_2::create(&dest_id)
        .container_format(ContainerFormat::Cmaf)
        .encryption(Encryption::Content {
            content_encryption: ContentEncryption {
                key: "00112233445566778899aabbccddeeff".to_string(),
                key_id: "00112233-4455-6677-8899-aabbccddeeff".to_string(),
            },
            protection_scheme: ProtectionScheme::CENC,
        })
        .send(&client)
        .await
        .unwrap();

    let fetched = apis::recording_group_2::get(created.id.clone())
        .send(&client)
        .await
        .unwrap();
    assert_eq!(fetched.id, created.id);
    assert!(matches!(
        fetched.encryption,
        Some(Encryption::Content { .. })
    ));

    apis::recording_group_2::delete(created.id)
        .send(&client)
        .await
        .unwrap();
    delete_destination(&client, &dest_id).await;
}

#[tokio::test]
async fn recording_group_2_crud_with_key_encryption() {
    let Some(client) = test_client().await else {
        return;
    };

    let dest_id = somewhat_unique_name("smoke_test_recording_destination_");

    apis::remote_object_storage_1::create_destinations()
        .data(json!({
            "id": dest_id,
            "s3": {
                "accessKeyId": "myAccessKeyId",
                "secretAccessKey": "mySecretAccessKey",
                "bucket": "myBucket",
                "url": "https://s3.eu-north-1.amazonaws.com",
            }
        }))
        .send(&client)
        .await
        .unwrap();

    let created = apis::recording_group_2::create(&dest_id)
        .container_format(ContainerFormat::Cmaf)
        .encryption(Encryption::Key {
            key_encryption: KeyEncryption {
                certificate_ids: None,
                key_rotation_duration: 3600,
                public_keys: Some(vec![PublicKey {
                    // Throwaway EC P-256 key generated with:
                    //   openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 | openssl pkey -pubout
                    key: "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAER6C4Swg6ki/GjOJFWklUoIN4+58f\nTTgv91yx0AQpL279e35DVCCl3CcNqd7M+raTVTuT3Om/iOt+RdV+XnHapg==\n-----END PUBLIC KEY-----".to_string(),
                    key_id: "aabbccdd-1122-3344-5566-778899aabbcc".to_string(),
                }]),
            },
            protection_scheme: ProtectionScheme::CENC,
        })
        .send(&client)
        .await
        .unwrap();

    let fetched = apis::recording_group_2::get(created.id.clone())
        .send(&client)
        .await
        .unwrap();
    assert_eq!(fetched.id, created.id);
    assert!(matches!(fetched.encryption, Some(Encryption::Key { .. })));

    apis::recording_group_2::delete(created.id)
        .send(&client)
        .await
        .unwrap();
    delete_destination(&client, &dest_id).await;
}

#[tokio::test]
async fn system_ready_system_ready_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };
    apis::system_ready_1::system_ready()
        .send(&client)
        .await
        .unwrap();
}

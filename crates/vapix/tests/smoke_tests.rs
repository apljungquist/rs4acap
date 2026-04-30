use std::{ops::Rem, time::SystemTime};

use log::LevelFilter;
use rs4a_vapix::{
    apis::{
        action1::{
            AddActionConfigurationRequest, AddActionRuleRequest, Condition,
            GetActionConfigurationsRequest, GetActionRulesRequest,
        },
        event1::GetEventInstancesRequest,
        recording_group_1::CreateRecordingGroupsRequest,
        remote_object_storage_1_beta::{CreateDestinationRequest, DestinationId, S3Destination},
        system_ready_1::SystemReadyRequest,
    },
    requests::jpg_3::GetImageRequest,
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
    Some(client.build().await.unwrap())
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
    GetActionConfigurationsRequest::new()
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
        AddActionConfigurationRequest::new("com.axis.action.fixed.ledcontrol")
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
        AddActionRuleRequest::new(action_rule_name.to_string(), action_configuration_id)
            .condition(Condition {
                topic_expression: "tns1:Device/tnsaxis:Status/SystemReady".to_string(),
                message_content: r#"boolean(//SimpleItem[@Name="ready" and @Value="1"])"#
                    .to_string(),
            })
            .send(&client)
            .await
            .unwrap()
            .id;

    let actions_rules = GetActionRulesRequest::new()
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
async fn event_1_get_event_instances_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };
    GetEventInstancesRequest::new().send(&client).await.unwrap();
}

#[tokio::test]
async fn jpg_get_image_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };
    GetImageRequest::new()
        .compression(100)
        .send(&client)
        .await
        .unwrap();
}

#[tokio::test]
async fn recording_group_1_create_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };

    let expected_recording_destination_id =
        somewhat_unique_name("smoke_test_recording_destination_");

    let actual_recording_destination_id = CreateDestinationRequest::s3(
        DestinationId::new(expected_recording_destination_id.clone()),
        S3Destination {
            bucket: "myBucket".to_string(),
            region: None,
            url: "https://s3.eu-north-1.amazonaws.com".to_string(),
            access_key_id: Some("myAccessKeyId".to_string()),
            secret_access_key: Some("mySecretAccessKey".to_string()),
            session_token: None,
        },
    )
    .send(&client)
    .await
    .unwrap()
    .id;

    assert_eq!(
        expected_recording_destination_id,
        actual_recording_destination_id.into_string()
    );

    CreateRecordingGroupsRequest::new()
        .data(json!({
            "destinations": [{
                "remoteObjectStorage":  {
                    "id": expected_recording_destination_id
                },
            }],
        }))
        .send(&client)
        .await
        .unwrap();
}

#[tokio::test]
async fn system_ready_system_ready_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };
    SystemReadyRequest::new().send(&client).await.unwrap();
}

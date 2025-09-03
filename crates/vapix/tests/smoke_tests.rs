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
async fn action_1_add_and_get_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };

    let action_configuration_id = client
        .action1()
        .add_action_configuration()
        .body(String::from(
            r#"
                <NewActionConfiguration>
                    <Name>Flash status LED</Name>
                    <TemplateToken>com.axis.action.fixed.ledcontrol</TemplateToken>
                    <Parameters>
                        <Parameter Name="led" Value="statusled"/>
                        <Parameter Name="color" Value="green,none"/>
                        <Parameter Name="duration" Value="1"/>
                        <Parameter Name="interval" Value="250"/>
                    </Parameters>
                </NewActionConfiguration>
               "#,
        ))
        .send()
        .await
        .unwrap()
        .configuration_id;

    let action_rule_name = "smoke test rule";
    let action_rule_id = client.action1().add_action_rule().body(format!(
        r#"
        <NewActionRule>
            <Name>{action_rule_name}</Name>
            <Enabled>true</Enabled>
            <Conditions>
                <Condition>
                    <TopicExpression
                            Dialect="http://docs.oasis-open.org/wsn/t-1/TopicExpression/Concrete"
                            xmlns="http://docs.oasis-open.org/wsn/b-2">tns1:Device/tnsaxis:Status/SystemReady</TopicExpression>
                    <MessageContent
                            Dialect="http://www.onvif.org/ver10/tev/messageContentFilter/ItemFilter"
                            xmlns="http://docs.oasis-open.org/wsn/b-2">boolean(//SimpleItem[@Name="ready" and @Value="1"])</MessageContent>
                </Condition>
            </Conditions>
            <PrimaryAction>{action_configuration_id}</PrimaryAction>
        </NewActionRule>
        "#
    )).send().await.unwrap().id;

    let actions_rules = client
        .action1()
        .get_action_rules()
        .send()
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
async fn event_1_get_event_instances_returns_ok() {
    let Some(client) = test_client().await else {
        return;
    };
    client.event1().get_event_instances().send().await.unwrap();
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

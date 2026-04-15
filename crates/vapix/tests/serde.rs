use std::time::Duration;

use expect_test::expect_file;
use rs4a_vapix::{
    action1::{AddActionConfigurationResponse, Condition},
    apis,
    basic_device_info_1::{AllPropertiesData, AllUnrestrictedPropertiesData, Architecture},
    firmware_management_1::UpgradeData,
    json_rpc::{parse_data, parse_data_lossless},
    soap::parse_soap,
    soap_http::SoapRequest,
    system_ready_1::SystemreadyData,
};

#[test]
fn can_deserialize_action_1_examples() {
    let text =
        include_str!("../src/services/action1/examples/add_action_configuration_response.xml");
    let data = parse_soap::<AddActionConfigurationResponse>(text).unwrap();
    assert_eq!(data.configuration_id, 1);
}

#[test]
fn can_deserialize_basic_device_info_1_examples() {
    let text = include_str!("../src/axis_cgi/basic_device_info_1/get_all_properties_1_0.json");
    let property_list = parse_data_lossless::<AllPropertiesData>(text)
        .unwrap()
        .property_list;
    assert_eq!(property_list.restricted.architecture, Architecture::Mips);
    assert_eq!(property_list.unrestricted.prod_variant, None);

    let text = include_str!(
        "../src/axis_cgi/basic_device_info_1/get_all_unrestricted_properties_2004_error_1_0.json"
    );
    parse_data_lossless::<AllUnrestrictedPropertiesData>(text).unwrap_err();
    // TODO: Expose error code
}

#[test]
fn can_deserialize_firmware_management_1_examples() {
    let text = include_str!("../src/axis_cgi/firmware_management_1/upgrade_1_0.json");
    let UpgradeData { .. } = parse_data_lossless::<UpgradeData>(text).unwrap();

    let text = include_str!("../src/axis_cgi/firmware_management_1/upgrade_409_error_1_0.json");
    parse_data_lossless::<UpgradeData>(text).unwrap_err();
    // TODO: Expose error code
}

#[test]
fn can_deserialize_system_ready_1_examples() {
    let text = include_str!("../src/axis_cgi/system_ready_1/system_ready_200.json");
    let data = parse_data::<SystemreadyData>(text).unwrap();
    assert!(!data.needsetup);
}

#[test]
fn can_deserialize_system_ready_1_preview_mode() {
    let text = include_str!("serde/system_ready_1_preview_mode.json");
    let data = parse_data::<SystemreadyData>(text).unwrap();
    assert_eq!(
        data.parse_preview_mode().unwrap(),
        Some(Duration::from_secs(7200))
    );
}

#[test]
fn can_serialize_action_1_requests() {
    expect_file!["./snapshots/add_action_configuration.xml"].assert_eq(
        &apis::action_1::add_action_configuration("com.axis.action.fixed.ledcontrol")
            .name("Flash status LED")
            .param("led", "statusled")
            .param("color", "green,none")
            .param("duration", "1")
            .param("interval", "250")
            .to_envelope()
            .unwrap(),
    );
    expect_file!["./snapshots/add_action_rule.xml"].assert_eq(
        &apis::action_1::add_action_rule("My Action Rule".to_string(), 123)
            .condition(Condition {
                topic_expression: "tns1:Device/tnsaxis:Status/SystemReady".to_string(),
                message_content: r#"boolean(//SimpleItem[@Name="ready" and @Value="1"])"#
                    .to_string(),
            })
            .to_envelope()
            .unwrap(),
    );
    expect_file!["./snapshots/get_action_configurations.xml"].assert_eq(
        &apis::action_1::get_action_configurations()
            .to_envelope()
            .unwrap(),
    );
    expect_file!["./snapshots/get_action_rules.xml"]
        .assert_eq(&apis::action_1::get_action_rules().to_envelope().unwrap());
}

use insta::assert_snapshot;
use rs4a_vapix::{
    action1::{AddActionConfigurationResponse, Condition},
    apis,
    basic_device_info_1::AllUnrestrictedPropertiesData,
    json_rpc::parse_data,
    soap::parse_soap,
    soap_http::SoapRequest,
    system_ready_1::{EnglishBoolean, SystemreadyData},
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
    let text = include_str!(
        "../src/axis_cgi/basic_device_info_1/get_all_unrestricted_properties_200.json"
    );
    let data = parse_data::<AllUnrestrictedPropertiesData>(text).unwrap();
    assert_eq!(data.property_list.version, "12.5.56");
}

#[test]
fn can_deserialize_system_ready_1_examples() {
    let text = include_str!("../src/axis_cgi/system_ready_1/system_ready_200.json");
    let data = parse_data::<SystemreadyData>(text).unwrap();
    assert!(matches!(data.needsetup, EnglishBoolean::No));
}

#[test]
fn can_serialize_action_1_requests() {
    assert_snapshot!(
        apis::action_1::add_action_configuration("com.axis.action.fixed.ledcontrol")
            .name("Flash status LED")
            .param("led", "statusled")
            .param("color", "green,none")
            .param("duration", "1")
            .param("interval", "250")
            .to_envelope()
            .unwrap()
    );
    assert_snapshot!(
        apis::action_1::add_action_rule("My Action Rule".to_string(), 123)
            .condition(Condition {
                topic_expression: "tns1:Device/tnsaxis:Status/SystemReady".to_string(),
                message_content: r#"boolean(//SimpleItem[@Name="ready" and @Value="1"])"#
                    .to_string()
            })
            .to_envelope()
            .unwrap()
    );
    assert_snapshot!(apis::action_1::get_action_configurations()
        .to_envelope()
        .unwrap());
    assert_snapshot!(apis::action_1::get_action_rules().to_envelope().unwrap());
}

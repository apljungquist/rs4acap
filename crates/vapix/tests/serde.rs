use insta::assert_snapshot;
use rs4a_vapix::{
    action1::{AddActionConfigurationResponse, Condition},
    apis,
    basic_device_info_1::{AllPropertiesData, AllUnrestrictedPropertiesData, Architecture},
    json_rpc::{parse_data, parse_data_lossless},
    rest,
    rest_http::RestHttp,
    soap::parse_soap,
    soap_http::SoapRequest,
    ssh_1::{AddUserResponse, SetUserResponse},
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
    let text = include_str!(
        "../src/axis_cgi/basic_device_info_1/get_all_unrestricted_properties_200.json"
    );
    let data = parse_data_lossless::<AllUnrestrictedPropertiesData>(text).unwrap();
    assert_eq!(data.property_list.version, "12.5.56");

    let text = include_str!("../src/axis_cgi/basic_device_info_1/get_all_properties_1_3.json");
    let property_list = parse_data_lossless::<AllPropertiesData>(text)
        .unwrap()
        .property_list;
    assert_eq!(property_list.restricted.architecture, Architecture::Armv7hf);
    assert_eq!(property_list.unrestricted.prod_variant, None);

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
fn can_deserialize_ssh_1_post_user_201_response_from_axis_os_11() {
    let text = include_str!("../src/config/ssh_1_examples/add_user_201_response.json");
    rest::parse_data::<AddUserResponse>(text).unwrap();
}

#[test]
fn can_deserialize_ssh_1_post_user_201_response_from_axis_os_12() {
    let text = include_str!("../src/config/ssh_1_examples/add_user_201_response_12_7_61.json");
    rest::parse_data::<AddUserResponse>(text).unwrap();
}

#[test]
fn can_deserialize_ssh_1_success_response() {
    let text = include_str!("../src/config/ssh_1_examples/set_user_200_response.json");
    rest::parse_data::<SetUserResponse>(text).unwrap();
}

#[test]
fn can_deserialize_ssh_1_error_response() {
    let text = include_str!("../src/config/ssh_1_examples/set_user_404_response.json");
    let error = rest::parse_data::<SetUserResponse>(text).unwrap_err();
    let error = error.downcast::<rest::Error>().unwrap();
    assert_eq!(error.code, 2);
}

#[test]
fn can_deserialize_system_ready_1_examples() {
    let text = include_str!("../src/axis_cgi/system_ready_1/system_ready_200.json");
    let data = parse_data::<SystemreadyData>(text).unwrap();
    assert!(!data.needsetup);
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

#[test]
fn can_serialize_ssh_1_add_user_requests() {
    let (path, data) = apis::ssh_1::add_user("Dalliard", "Good morning")
        .to_path_and_data()
        .unwrap();
    assert_snapshot!(path);
    assert_snapshot!(serde_json::to_string(&data).unwrap());
}

#[test]
fn can_serialize_ssh_1_set_user_requests() {
    let (path, data) = apis::ssh_1::set_user("Dalliard")
        .comment("When's the day?")
        .to_path_and_data()
        .unwrap();
    assert_snapshot!(path);
    assert_snapshot!(serde_json::to_string(&data).unwrap());
}

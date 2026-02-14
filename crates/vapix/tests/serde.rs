use insta::assert_snapshot;
use rs4a_vapix::{
    action1::{AddActionConfigurationResponse, Condition},
    apis,
    basic_device_info_1::{AllPropertiesData, AllUnrestrictedPropertiesData, Architecture},
    json_rpc::{parse_data, parse_data_lossless},
    recording_group_2::{ContainerFormat, Encryption, ProtectionScheme, RecordingGroup},
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

#[test]
fn can_deserialize_recording_group_2_list_response() {
    let text = include_str!("../src/config/recording_group_2_examples/list_200_response.json");
    let groups = rest::parse_data::<Vec<RecordingGroup>>(text).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].id.as_ref(), "20260120_175850_CDD3D355");
    assert_eq!(groups[0].container_format, ContainerFormat::Cmaf);
    assert_eq!(groups[0].max_retention_time, 0);
}

#[test]
fn can_deserialize_recording_group_2_get_response() {
    let text = include_str!("../src/config/recording_group_2_examples/get_200_response.json");
    let group = rest::parse_data::<RecordingGroup>(text).unwrap();
    assert_eq!(group.id.as_ref(), "20260214_231153_182CB254");
    assert_eq!(group.nice_name, "");
    assert_eq!(group.destinations.len(), 1);
    assert_eq!(
        group.destinations[0].remote_object_storage.id,
        "smoke_test_recording_destination_256307"
    );
}

#[test]
fn can_deserialize_recording_group_2_create_response() {
    let text = include_str!("../src/config/recording_group_2_examples/create_201_response.json");
    let group = rest::parse_data::<RecordingGroup>(text).unwrap();
    assert_eq!(group.id.as_ref(), "20260214_231153_182CB254");
}

#[test]
fn can_deserialize_recording_group_2_content_encryption() {
    let text = include_str!(
        "../src/config/recording_group_2_examples/get_200_content_encryption_response.json"
    );
    let group = rest::parse_data::<RecordingGroup>(text).unwrap();
    let Encryption::Content {
        content_encryption,
        protection_scheme,
    } = group.encryption.as_ref().unwrap()
    else {
        panic!("expected content encryption");
    };
    assert_eq!(protection_scheme, &ProtectionScheme::CENC);
    assert_eq!(
        content_encryption.key_id,
        "00112233-4455-6677-8899-aabbccddeeff"
    );
}

#[test]
fn can_deserialize_recording_group_2_key_encryption() {
    let text = include_str!(
        "../src/config/recording_group_2_examples/get_200_key_encryption_response.json"
    );
    let group = rest::parse_data::<RecordingGroup>(text).unwrap();
    let Encryption::Key {
        key_encryption,
        protection_scheme,
    } = group.encryption.as_ref().unwrap()
    else {
        panic!("expected key encryption");
    };
    assert_eq!(protection_scheme, &ProtectionScheme::CENC);
    assert_eq!(key_encryption.key_rotation_duration, 3600);
    let public_keys = key_encryption.public_keys.as_ref().unwrap();
    assert_eq!(public_keys.len(), 1);
    assert_eq!(
        public_keys[0].key_id,
        "aabbccdd-1122-3344-5566-778899aabbcc"
    );
}

#[test]
fn can_deserialize_recording_group_2_delete_response() {
    let text = include_str!("../src/config/recording_group_2_examples/delete_200_response.json");
    let _: serde_json::Value = rest::parse_data(text).unwrap();
}

// If I remember correctly, this pattern of test was added to make sure that the generated requests
// remain valid when the code is refactored. This intention is easily lost with the current naming
// scheme.
// TODO: Consider renaming `can_serialize_*` tests
#[test]
fn can_serialize_recording_group_2_create_request() {
    let (path, data) = apis::recording_group_2::create("my-dest")
        .nice_name("Test Group")
        .container_format(ContainerFormat::Matroska)
        .to_path_and_data()
        .unwrap();
    assert_snapshot!(path);
    assert_snapshot!(serde_json::to_string(&data).unwrap());
}

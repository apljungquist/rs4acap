use rs4a_vapix::{
    basic_device_info_1::AllUnrestrictedPropertiesData,
    json_rpc::parse_data,
    system_ready_1::{EnglishBoolean, SystemreadyData},
};

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

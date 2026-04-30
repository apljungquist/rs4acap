use std::{future::Future, pin::Pin};

use anyhow::Context;
use libtest_mimic::{Arguments, Trial};
use log::{warn, LevelFilter};
use rs4a_cassette_testing::{Cassette, CassetteClient, DeviceInfo, Library};
use rs4a_vapix::{
    apis::{
        action1::{GetActionConfigurationsRequest, GetActionRulesRequest},
        api_discovery_1::{Api, ApiListData, GetApiListRequest},
        basic_device_info_1::{
            GetAllPropertiesRequest, GetAllUnrestrictedPropertiesRequest, ProductType,
            UnrestrictedProperties,
        },
        discover::ApiState,
        event1::GetEventInstancesRequest,
        firmware_management_1,
        firmware_management_1::UpgradeRequest,
        network_settings_1::{GetNetworkInfoRequest, SetGlobalProxyConfigurationRequest},
        parameter_management::{ImageResolution, ListRequest, NetworkSshEnabled, UpdateRequest},
        remote_object_storage_1_beta::{
            AzureDestination, CreateDestinationRequest, DeleteDestinationRequest, DestinationData,
            DestinationId, ListDestinationsRequest, UpdateDestinationRequest,
        },
        siren_and_light_2_alpha::{
            GetMaintenanceModeRequest, StartMaintenanceModeRequest, StopMaintenanceModeRequest,
        },
        system_ready_1::SystemReadyRequest,
    },
    protocol_helpers::{http, rest::ErrorKind},
    ClientBuilder,
};
use semver::VersionReq;
use url::Url;
// When a test fails, it may leave resources intact that will cause future runs to fail.
// This must be cleaned up manually by either removing them individually or resetting the device.
// This is tedious, but hopefully updating cassettes will be rare.
// TODO: Avoid manual cleanup.

// When comparing cassettes, it is difficult to know where they are different.
// For example, in a test like `device_configuration_item_already_exists` the first two responses
// are the same on AXIS OS 11 and 12, but the third is different.
// This could be alleviated by:
// - Including the response hash in the response name
// - Provide a diff tool
// TODO: Make it easier to compare cassettes.

// For most APIs it does not make sense to support minor versions other than the latest.
// The main exceptions are APIs needed for device re-init and upgrade.
// TODO: Clean up superseded cassettes automatically.

// If a test is removed, the cassettes must be removed manually.
// TODO: Clean up obsolete shelves automatically.

// Some responses cannot be triggered with a well-behaved client.
// Examples include:
// - Bad content type (e.g. by omitting the content-type header on AXIS OS 11)
// - Resource not found (e.g. by requesting siren and light on P8815)
// TODO: Figure out how to test unhappy paths.

// On AXIS OS without the device config API, the response status is 404, which parses into a
// decoding error.
// TODO: Consider making the lack of the API more explicit

pub fn env_flag(key: &str) -> bool {
    match std::env::var(key).as_deref() {
        Ok("0") => false,
        Ok("1") => true,
        Ok(s) => panic!("Expected value '0' or '1' but found '{s}' for {key}"),
        Err(_) => false,
    }
}

#[derive(Clone, Debug)]
struct Prelude {
    props: UnrestrictedProperties,
    api_list: ApiListData,
}

impl Prelude {
    pub(crate) fn supports_device_config(&self) -> bool {
        self.version_matches(">=11")
    }

    pub(crate) fn is_supported(
        &self,
        id: rs4a_vapix::apis::api_discovery_1::ApiId,
        req: &str,
    ) -> bool {
        self.api_list.is_supported(id, req).unwrap()
    }

    fn version_matches(&self, req: &str) -> bool {
        let v = self.props.parse_version().unwrap();
        let req = VersionReq::parse(req).unwrap();
        req.matches(&v)
    }
}

type TestFn = for<'a> fn(
    &'a CassetteClient,
    Option<Prelude>,
) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
type Substitutions = &'static [(&'static str, &'static str)];
type TestEntry = (&'static str, TestFn, Substitutions);

macro_rules! cassette_tests {
    (@entry $name:ident => [$($sub:expr),* $(,)?]) => {
        (
            stringify!($name),
            (|client, prelude| {
                Box::pin($name(client, prelude))
            }) as TestFn,
            &[$($sub),*] as Substitutions,
        )
    };
    (@entry $name:ident) => {
        cassette_tests!(@entry $name => [])
    };
    ($($name:ident $(=> [$($sub:expr),* $(,)?])?),* $(,)?) => {
        const TESTS: &[TestEntry] = &[
            $(cassette_tests!(@entry $name $(=> [$($sub),*])?),)*
        ];
    };
}

cassette_tests! {
    action1_get_action_configurations,
    action1_get_action_rules,
    api_discovery_1_get_api_list,
    api_discovery_1_get_supported_versions,
    network_settings_1_get_network_info => [
        // MAC address
        (
            r#""macAddress": "[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}""#,
            r#""macAddress": "01:23:45:67:89:ab""#,
        ),
        // IPv6 link-local (EUI-64, derived from MAC)
        (
            r#""address": "fe80::[0-9a-f]{4}:[0-9a-f]{4}:[0-9a-f]{4}:[0-9a-f]{4}""#,
            r#""address": "fe80::0123:4567:89ab:cdef""#,
        ),
        // Hostname derived from MAC/serial
        (
            r#""hostname": "axis-[0-9a-f]{12}""#,
            r#""hostname": "axis-0123456789ab""#,
        ),
        (
            r#""staticHostname": "axis-[0-9a-f]{12}""#,
            r#""staticHostname": "axis-0123456789ab""#,
        ),
        (
            r#""identity": "axis-[0-9a-f]{12}""#,
            r#""identity": "axis-0123456789ab""#,
        ),
    ],
    network_settings_1_set_global_proxy_configuration => [
        // MAC address
        (
            r#""macAddress": "[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}:[0-9a-f]{2}""#,
            r#""macAddress": "01:23:45:67:89:ab""#,
        ),
        // IPv6 link-local (EUI-64, derived from MAC)
        (
            r#""address": "fe80::[0-9a-f]{4}:[0-9a-f]{4}:[0-9a-f]{4}:[0-9a-f]{4}""#,
            r#""address": "fe80::0123:4567:89ab:cdef""#,
        ),
        // Hostname derived from MAC/serial
        (
            r#""hostname": "axis-[0-9a-f]{12}""#,
            r#""hostname": "axis-0123456789ab""#,
        ),
        (
            r#""staticHostname": "axis-[0-9a-f]{12}""#,
            r#""staticHostname": "axis-0123456789ab""#,
        ),
        (
            r#""identity": "axis-[0-9a-f]{12}""#,
            r#""identity": "axis-0123456789ab""#,
        ),
    ],
    basic_device_info_get_all_properties => [
        (
            r#""SocSerialNumber": "[0-9A-F]{8}-[0-9A-F]{8}-[0-9A-F]{8}-[0-9A-F]{8}""#,
            r#""SocSerialNumber": "00000000-00000000-01234567-89ABCDEF""#,
        ),
        (
            r#""SocSerialNumber": "[0-9A-F]{8}-[0-9A-F]{8}""#,
            r#""SocSerialNumber": "01234567-89ABCDEF""#,
        ),
        (
            r#""SocSerialNumber": "[0-9A-F]{16}""#,
            r#""SocSerialNumber": "0123456789ABCDEF""#,
        ),
        (
            r#""SerialNumber": "[0-9A-F]{12}""#,
            r#""SerialNumber": "0123456789AB""#,
        ),
    ],
    basic_device_info_get_all_unrestricted_properties => [
        (
            r#""SerialNumber": "[0-9A-F]{12}""#,
            r#""SerialNumber": "0123456789AB""#,
        ),
    ],
    device_configuration_discover,
    device_configuration_item_does_not_exist,
    device_configuration_validation_error,
    device_configuration_item_already_exists,
    event1_get_event_instances => [
        (
            r#"Name="DeviceUUID"><aev:Value>[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}</aev:Value>"#,
            r#"Name="DeviceUUID"><aev:Value>00000000-0000-0000-0123-456789abcdef</aev:Value>"#,
        ),
    ],
    firmware_management_1_upgrade_mismatch,
    parameter_management_list_error,
    parameter_management_list_image_resolution,
    parameter_management_update_network_ssh_enabled,
    pwdgrp_add_user_already_exists,
    pwdgrp_add_user_invalid_password,
    pwdgrp_add_user_invalid_username,
    pwdgrp_remove_user_does_not_exist,
    remote_object_storage_1_beta_crud,
    siren_and_light_2_alpha_maintenance_mode_not_supported,
    ssh_1_crud,
    ssh_1_set_user_does_not_exist,
    ssh_1_set_user_validation_error,
    system_ready_1_system_ready => [
        (
            r#""uptime": "\d+""#,
            r#""uptime": "0""#
        ),
        (
            r#""bootid": "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}""#,
            r#""bootid": "00000000-0000-0000-0000-000000000000""#,
        ),
    ],
}

fn record_trials(library: &Library) -> Vec<Trial> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let client = rt.block_on(async {
        ClientBuilder::from_dut()
            .unwrap()
            .unwrap()
            .build()
            .await
            .unwrap()
    });

    let prelude = rt.block_on(async {
        let props = GetAllUnrestrictedPropertiesRequest::new()
            .send(&client)
            .await
            .unwrap()
            .property_list;
        let api_list = GetApiListRequest::default().send(&client).await.unwrap();
        Prelude { props, api_list }
    });

    let device_info = DeviceInfo {
        prod_nbr: prelude.props.prod_nbr.clone(),
        version: prelude.props.version.clone(),
    };

    TESTS
        .iter()
        .map(|&(test_name, test_fn, substitutions)| {
            let cassette = Cassette::new(substitutions);
            let cassette_client = CassetteClient::for_recording(client.clone(), cassette);
            let prelude = prelude.clone();
            let library = library.clone();
            let device_info = device_info.clone();

            Trial::test(test_name, move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                let () = rt.block_on(test_fn(&cassette_client, Some(prelude)));
                let () =
                    library.write(test_name, &device_info, &cassette_client.take_cassette())?;
                Ok(())
            })
        })
        .collect()
}

fn playback_trials(library: &Library) -> Vec<Trial> {
    let mut cassettes = library.cassettes().unwrap();
    let mut trials = Vec::new();

    for &(test_name, test_fn, _) in TESTS {
        let Some(variants) = cassettes.remove(test_name) else {
            trials.push(Trial::test(test_name, || Ok(())).with_ignored_flag(true));
            continue;
        };
        for (label, cassette) in variants {
            let trial_name = format!("{test_name}::{label}");
            match cassette {
                Some(cassette) => trials.push(Trial::test(trial_name, move || {
                    let cassette_client = CassetteClient::for_playback(cassette);
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    let () = rt.block_on(test_fn(&cassette_client, None));
                    Ok(())
                })),
                None => trials.push(Trial::test(trial_name, || Ok(())).with_ignored_flag(true)),
            }
        }
    }

    if !cassettes.is_empty() {
        warn!("Found {} with no corresponding test", cassettes.len());
    }

    trials.sort_by(|lhs, rhs| lhs.name().cmp(rhs.name()));
    trials
}

fn main() {
    let _ = env_logger::Builder::new()
        .filter_level(LevelFilter::Warn)
        .parse_default_env()
        .is_test(true)
        .try_init();

    let mut args = Arguments::from_args();
    let library = Library::new().unwrap();

    let update = env_flag("UPDATE_CASSETTES");
    let trials = match update {
        true => record_trials(&library),
        false => playback_trials(&library),
    };

    if args.test_threads.is_none() {
        println!("Running tests in single-threaded mode");
        args.test_threads = Some(1);
    }

    let conclusion = libtest_mimic::run(&args, trials);
    if update {
        library.cleanup_unreferenced().unwrap();
    }
    conclusion.exit();
}

async fn action1_get_action_configurations(client: &CassetteClient, _prelude: Option<Prelude>) {
    GetActionConfigurationsRequest.send(client).await.unwrap();
}

async fn action1_get_action_rules(client: &CassetteClient, _prelude: Option<Prelude>) {
    GetActionRulesRequest.send(client).await.unwrap();
}

async fn api_discovery_1_get_api_list(client: &CassetteClient, _: Option<Prelude>) {
    use rs4a_vapix::apis::{api_discovery_1::GetApiListRequest, network_settings_1};

    let data = GetApiListRequest::default().send(client).await.unwrap();
    let Api { .. } = data
        .api_list
        .iter()
        .find(|a| a.id == "api-discovery" && a.parse_version().unwrap().major == 1)
        .expect("api-discovery should be in its own list");

    for api in &data.api_list {
        api.parse_version().unwrap();
        api.parse_status().unwrap();
    }

    assert!(data
        .is_supported(network_settings_1::API_ID, ">=1")
        .unwrap());
}

async fn api_discovery_1_get_supported_versions(client: &CassetteClient, _: Option<Prelude>) {
    use rs4a_vapix::apis::api_discovery_1::GetSupportedVersionsRequest;

    let data = GetSupportedVersionsRequest::default()
        .send(client)
        .await
        .unwrap();
    assert!(!data.api_versions.is_empty());
}

async fn basic_device_info_get_all_properties(client: &CassetteClient, _: Option<Prelude>) {
    let property_list = GetAllPropertiesRequest::new()
        .send(client)
        .await
        .unwrap()
        .property_list;

    let _ = property_list
        .restricted
        .parse_soc_serial_number()
        .context(format!("{:?}", property_list.restricted.soc_serial_number))
        .unwrap();
}

async fn basic_device_info_get_all_unrestricted_properties(
    client: &CassetteClient,
    _: Option<Prelude>,
) {
    let property_list = GetAllUnrestrictedPropertiesRequest::new()
        .send(client)
        .await
        .unwrap()
        .property_list;

    property_list.parse_product_type().unwrap();
    property_list.parse_version().unwrap();
}

async fn device_configuration_discover(client: &CassetteClient, prelude: Option<Prelude>) {
    use rs4a_vapix::apis::discover::DiscoverRequest;

    if let Some(prelude) = prelude {
        if !prelude.supports_device_config() {
            return;
        }
    }

    let data = DiscoverRequest.send(client).await.unwrap();
    assert!(!data.apis.is_empty());

    for versions in data.apis.values() {
        for info in versions.values() {
            let pre = info.version.pre.as_str();
            let state = info.parse_state().unwrap();

            match state {
                ApiState::Alpha | ApiState::Beta => {
                    assert!(pre.starts_with(state.to_string().as_str()))
                }
                ApiState::Released => assert!(pre.is_empty()),
                _ => todo!("{state:?}"),
            }
        }
    }
}

async fn device_configuration_item_does_not_exist(
    client: &CassetteClient,
    prelude: Option<Prelude>,
) {
    if let Some(prelude) = prelude {
        if !prelude.supports_device_config() {
            return;
        }
    }

    let id = DestinationId::new("my_destination_id".to_string());

    let error = DeleteDestinationRequest::new(id.clone())
        .send(client)
        .await
        .unwrap_err();

    let http::Error::Service(error) = error else {
        panic!("Expected Service error but got {error:?}");
    };

    assert_eq!(error.kind().unwrap(), ErrorKind::NotFound);
}

async fn device_configuration_validation_error(client: &CassetteClient, prelude: Option<Prelude>) {
    if let Some(prelude) = prelude {
        if !prelude.supports_device_config() {
            return;
        }
    }

    let error = CreateDestinationRequest::azure(
        DestinationId::new("my_destination_id".to_string()),
        AzureDestination::new(
            "my-container".to_string(),
            "".to_string(),
            Url::parse("https://s3.eu-north-1.amazonaws.com").unwrap(),
        ),
    )
    .send(client)
    .await
    .unwrap_err();

    let http::Error::Service(error) = error else {
        panic!("Expected Service error but got {error:?}");
    };

    assert_eq!(error.kind().unwrap(), ErrorKind::ValidationError);
}

async fn device_configuration_item_already_exists(
    client: &CassetteClient,
    prelude: Option<Prelude>,
) {
    if let Some(prelude) = prelude {
        if !prelude.supports_device_config() {
            return;
        }
    }

    let id = DestinationId::new("my_destination_id".to_string());

    let DestinationData { .. } = CreateDestinationRequest::azure(
        id.clone(),
        AzureDestination::new(
            "my-container".to_string(),
            "my-sas".to_string(),
            Url::parse("https://s3.eu-north-1.amazonaws.com").unwrap(),
        ),
    )
    .send(client)
    .await
    .unwrap();

    let error = CreateDestinationRequest::azure(
        id.clone(),
        AzureDestination::new(
            "my-container".to_string(),
            "my-sas".to_string(),
            Url::parse("https://s3.eu-north-1.amazonaws.com").unwrap(),
        ),
    )
    .send(client)
    .await
    .unwrap_err();

    let http::Error::Service(error) = error else {
        panic!("Expected Service error but got {error:?}");
    };

    assert_eq!(error.kind().unwrap(), ErrorKind::AlreadyExists);

    // Cleanup

    // Leaving the destination behind may affect other tests, even if we didn't reuse it.
    // But we don't really need this last track on the cassette.
    // TODO: Consider running this only when recording and excluding it from the cassette.

    DeleteDestinationRequest::new(id.clone())
        .send(client)
        .await
        .unwrap();
}

// TODO: Find a way to avoid the churn caused by XML lists not being consistently ordered
async fn event1_get_event_instances(client: &CassetteClient, _prelude: Option<Prelude>) {
    let data = GetEventInstancesRequest.send(client).await.unwrap();
    assert!(!data.message_instances.is_empty());
}

// This normally happens if the firmware is for a different device model.
// Apparently it also happens with an invalid firmware binary.
async fn firmware_management_1_upgrade_mismatch(
    client: &CassetteClient,
    _prelude: Option<Prelude>,
) {
    let firmware = b"DUMMY_FIRMWARE_BYTES".to_vec();
    let error = UpgradeRequest::new(firmware)
        .send(client)
        .await
        .unwrap_err();

    let http::Error::Service(error) = error else {
        panic!("Expected Service error but got {error:?}");
    };

    assert_eq!(
        firmware_management_1::ErrorKind::try_from(error.code),
        Ok(firmware_management_1::ErrorKind::ImageMismatch),
    );
}

async fn parameter_management_list_error(client: &CassetteClient, prelude: Option<Prelude>) {
    if let Some(prelude) = prelude {
        match prelude.props.parse_product_type().unwrap() {
            ProductType::BoxCamera => return,
            ProductType::DomeCamera => return,
            ProductType::NetworkCamera => return,
            ProductType::NetworkStrobeSpeaker => {}
            ProductType::Radar => return,
            ProductType::PeopleCounter3D => return,
            ProductType::ThermalCamera => return,
            _ => {}
        }
    }

    let error = ListRequest::new::<ImageResolution>()
        .send(client)
        .await
        .unwrap_err();

    // TODO: Parse the error code and message
    assert!(error.to_string().contains("-1"));
}

async fn parameter_management_list_image_resolution(
    client: &CassetteClient,
    prelude: Option<Prelude>,
) {
    if let Some(prelude) = prelude {
        if matches!(
            prelude.props.parse_product_type().unwrap(),
            ProductType::AirQualitySensor | ProductType::NetworkStrobeSpeaker
        ) {
            return;
        }
    }

    let params = ListRequest::new::<ImageResolution>()
        .send(client)
        .await
        .unwrap();
    let resolutions = params.parse::<ImageResolution>().unwrap().unwrap();
    assert!(!resolutions.is_empty());
}

async fn parameter_management_update_network_ssh_enabled(
    client: &CassetteClient,
    _prelude: Option<Prelude>,
) {
    let read = || async {
        ListRequest::new::<NetworkSshEnabled>()
            .send(client)
            .await
            .unwrap()
            .parse::<NetworkSshEnabled>()
            .unwrap()
            .unwrap()
    };

    let initial = read().await;

    UpdateRequest::default()
        .network_ssh_enabled(!initial)
        .send(client)
        .await
        .unwrap();

    assert_eq!(read().await, !initial);

    UpdateRequest::default()
        .network_ssh_enabled(initial)
        .send(client)
        .await
        .unwrap();

    assert_eq!(read().await, initial);
}

async fn pwdgrp_add_user_already_exists(client: &CassetteClient, _prelude: Option<Prelude>) {
    use rs4a_vapix::apis::pwdgrp::{AddUserRequest, Group, RemoveUserRequest, Role};
    let username = "cassettetest";

    AddUserRequest::new(username, "Good morning", Group::Users, Role::Viewer)
        .send(client)
        .await
        .unwrap();

    let err = AddUserRequest::new(username, "Good morning", Group::Users, Role::Viewer)
        .send(client)
        .await
        .unwrap_err();

    let http::Error::Service(err) = err else {
        panic!("Expected Service error but got {err:?}");
    };
    assert_eq!(
        err.message(),
        "this user name already exists, consult the system log file"
    );

    // Cleanup
    RemoveUserRequest::new(username).send(client).await.unwrap();
}

async fn pwdgrp_add_user_invalid_password(client: &CassetteClient, _prelude: Option<Prelude>) {
    use rs4a_vapix::apis::pwdgrp::{AddUserRequest, Group, Role};

    let err = AddUserRequest::new("testuser", "", Group::Users, Role::Viewer)
        .send(client)
        .await
        .unwrap_err();

    let http::Error::Service(err) = err else {
        panic!("Expected Service error but got {err:?}");
    };
    assert_eq!(err.message(), "invalid password");
}

async fn pwdgrp_add_user_invalid_username(client: &CassetteClient, _prelude: Option<Prelude>) {
    use rs4a_vapix::apis::pwdgrp::{AddUserRequest, Group, Role};

    let err = AddUserRequest::new("user!", "Good morning", Group::Users, Role::Viewer)
        .send(client)
        .await
        .unwrap_err();

    let http::Error::Service(err) = err else {
        panic!("Expected Service error but got {err:?}");
    };
    assert_eq!(err.message(), "account user name");
}

async fn pwdgrp_remove_user_does_not_exist(client: &CassetteClient, _prelude: Option<Prelude>) {
    use rs4a_vapix::apis::pwdgrp::RemoveUserRequest;

    let err = RemoveUserRequest::new("nonexistent_user")
        .send(client)
        .await
        .unwrap_err();

    let http::Error::Service(err) = err else {
        panic!("Expected Service error but got {err:?}");
    };
    assert_eq!(err.message(), "account user name");
}

async fn remote_object_storage_1_beta_crud(client: &CassetteClient, prelude: Option<Prelude>) {
    if let Some(prelude) = prelude {
        if !prelude.supports_device_config() {
            return;
        }
    }

    let id = DestinationId::new("my_destination_id".to_string());

    // Create
    let created = CreateDestinationRequest::azure(
        id.clone(),
        AzureDestination::new(
            "my-container".to_string(),
            "my-sas".to_string(),
            Url::parse("https://s3.eu-north-1.amazonaws.com").unwrap(),
        ),
    )
    .description("my-description".to_string())
    .send(client)
    .await
    .unwrap();
    assert_eq!(&created.id, &id);
    assert!(created.azure.is_some());

    // List
    let all = ListDestinationsRequest::new().send(client).await.unwrap();
    assert!(all.iter().any(|d| d.id == created.id));
    assert_eq!(
        all.len(),
        1,
        "Expected exactly one destination in the list, but found {all:#?}"
    );

    // On at least one occasion when recording, this returned an item does not exist error.
    // TODO: Consider retrying the request if the destination is not yet available.

    // Update
    let () = UpdateDestinationRequest::azure(
        created.id.clone(),
        AzureDestination {
            sas: Some("my-updated-sas".to_string()),
            ..created.azure.unwrap()
        },
    )
    .send(client)
    .await
    .unwrap();
    // The effect of this update cannot be observed since the sas is redacted.

    let updated_description = format!("{}-updated", created.description.unwrap());
    let () = UpdateDestinationRequest::description(created.id.clone(), updated_description.clone())
        .send(client)
        .await
        .unwrap();
    let all = ListDestinationsRequest::new().send(client).await.unwrap();
    let updated = all.into_iter().find(|d| d.id == created.id).unwrap();
    assert_eq!(updated.description.unwrap(), updated_description);

    // Delete
    let () = DeleteDestinationRequest::new(created.id.clone())
        .send(client)
        .await
        .unwrap();

    // Verify deletion
    let all = ListDestinationsRequest::new().send(client).await.unwrap();
    assert!(!all.iter().any(|d| d.id == created.id));
}

// TODO: Figure out why these recordings are inconsistent
async fn siren_and_light_2_alpha_maintenance_mode_not_supported(
    client: &CassetteClient,
    prelude: Option<Prelude>,
) {
    // TODO: Use the config discovery API to get capabilities and make this more robust.
    if let Some(prelude) = prelude {
        if !prelude.supports_device_config() {
            return;
        }
        if !prelude.version_matches(">=12.5.0") {
            return;
        }
        if ["M1075-L", "D2210-VE", "Q1961-TE"].contains(&prelude.props.prod_nbr.as_str()) {
            return;
        }
    }

    GetMaintenanceModeRequest::new().send(client).await.unwrap();

    let error = StopMaintenanceModeRequest::new()
        .send(client)
        .await
        .unwrap_err();

    let http::Error::Service(error) = error else {
        panic!("Expected Service error but got {error:?}");
    };

    assert_eq!(error.kind(), Some(ErrorKind::InternalError));

    let error = StartMaintenanceModeRequest::new()
        .send(client)
        .await
        .unwrap_err();

    let http::Error::Service(error) = error else {
        panic!("Expected Service error but got {error:?}");
    };

    assert_eq!(error.kind(), Some(ErrorKind::InternalError));
}

async fn ssh_1_crud(client: &CassetteClient, prelude: Option<Prelude>) {
    use rs4a_vapix::apis::ssh_1::{AddUserRequest, DeleteUserRequest, SetUserRequest};

    if let Some(prelude) = &prelude {
        if !prelude.supports_device_config() {
            return;
        }
    }

    let username = "dalliard";

    AddUserRequest::new(username, "Good morning")
        .comment("Good morning")
        .send(client)
        .await
        .unwrap();

    SetUserRequest::new(username)
        .comment("When's the day?")
        .send(client)
        .await
        .unwrap();

    DeleteUserRequest::new(username).send(client).await.unwrap();
}

async fn ssh_1_set_user_does_not_exist(client: &CassetteClient, prelude: Option<Prelude>) {
    use rs4a_vapix::apis::ssh_1::SetUserRequest;
    if let Some(prelude) = &prelude {
        if !prelude.supports_device_config() {
            return;
        }
    }

    let error = SetUserRequest::new("nonexistent_user")
        .comment("should fail")
        .send(client)
        .await
        .unwrap_err();

    let http::Error::Service(error) = error else {
        panic!("Expected Service error but got {error:?}");
    };

    assert_eq!(error.kind().unwrap(), ErrorKind::NotFound);
}

async fn ssh_1_set_user_validation_error(client: &CassetteClient, prelude: Option<Prelude>) {
    use rs4a_vapix::apis::ssh_1::{AddUserRequest, DeleteUserRequest, SetUserRequest};

    if let Some(prelude) = &prelude {
        if !prelude.supports_device_config() {
            return;
        }
    }

    let username = "cassette_test_validation";

    AddUserRequest::new(username, "Good morning")
        .comment("Good morning")
        .send(client)
        .await
        .unwrap();

    // Empty string violates the minimum length of 1
    let error = SetUserRequest::new(username)
        .password("")
        .send(client)
        .await
        .unwrap_err();

    let http::Error::Service(error) = error else {
        panic!("Expected Service error but got {error:?}");
    };

    assert_eq!(error.kind().unwrap(), ErrorKind::ValidationError);

    // Clean up
    DeleteUserRequest::new(username).send(client).await.unwrap();
}

async fn network_settings_1_get_network_info(client: &CassetteClient, prelude: Option<Prelude>) {
    if let Some(prelude) = prelude {
        if prelude.is_supported(rs4a_vapix::apis::network_settings_1::API_ID, ">=1.33") {
            return;
        }
    }

    let data = GetNetworkInfoRequest::new().send(client).await.unwrap();
    assert!(data.system.global_proxies.is_none(),);
}

async fn network_settings_1_set_global_proxy_configuration(
    client: &CassetteClient,
    prelude: Option<Prelude>,
) {
    if let Some(prelude) = prelude {
        if prelude.is_supported(rs4a_vapix::apis::network_settings_1::API_ID, "<1.33") {
            return;
        }
    }

    let read = || async {
        let data = GetNetworkInfoRequest::new().send(client).await.unwrap();
        data.system.global_proxies.unwrap()
    };

    let initial = read().await;

    SetGlobalProxyConfigurationRequest::new()
        .http_proxy("http://192.0.2.1:8080")
        .https_proxy("http://192.0.2.1:8080")
        .no_proxy("192.0.2.2")
        .send(client)
        .await
        .unwrap();

    let updated = read().await;
    assert_eq!(updated.http_proxy, "http://192.0.2.1:8080");
    assert_eq!(updated.https_proxy, "http://192.0.2.1:8080");
    assert_eq!(updated.no_proxy, "192.0.2.2");

    SetGlobalProxyConfigurationRequest::new()
        .http_proxy(&initial.http_proxy)
        .https_proxy(&initial.https_proxy)
        .no_proxy(&initial.no_proxy)
        .send(client)
        .await
        .unwrap();

    let restored = read().await;
    assert_eq!(restored.http_proxy, initial.http_proxy);
    assert_eq!(restored.https_proxy, initial.https_proxy);
    assert_eq!(restored.no_proxy, initial.no_proxy);
}

async fn system_ready_1_system_ready(client: &CassetteClient, _prelude: Option<Prelude>) {
    let data = SystemReadyRequest::new().send(client).await.unwrap();
    assert!(data.systemready);
}

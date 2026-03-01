use std::{fs, path::PathBuf};

use anyhow::Context;
use log::LevelFilter;
use rs4a_vapix::{
    apis,
    basic_device_info_1::UnrestrictedProperties,
    cassette::{Cassette, Mode},
    http,
    json_rpc_http::JsonRpcHttp,
    recording_group_2::{
        CreateRecordingGroupRequest, DeleteRecordingGroupRequest, ListRecordingGroupsRequest,
    },
    remote_object_storage_1_beta::{
        AzureDestination, CreateDestinationRequest, DeleteDestinationRequest, DestinationData,
        DestinationId, ListDestinationsRequest, UpdateDestinationRequest,
    },
    rest::ErrorKind,
    rest_http2::RestHttp2,
    Client, ClientBuilder, Scheme,
};
use url::{Host, Url};
// Responses will vary depending on hardware model and software version.
// Creating a new shelf for each would be unwieldy.
// TODO: Automatically deduplicate shelves.

// Responses may vary due to time, initial state of the device, test order, etc.
// Manually determining when cassettes should be updated is tedious and error prone.
// Furthermore, if cassettes are addressed by their content, it cache hits will suffer.
// TODO: Automatically deal with non-reproducible responses.

// Not all tests are applicable to all hardware models and all software versions.
// TODO: Allow tests to filter on device capabilities.

// Keeping track of which shelves pass, fail, or are skipped is difficult when using a loop.
// TODO: Find a way to report the results from each cassette.

// When a test fails, it may leave resources intact that will cause future runs to fail.
// This must be cleaned up manually by either removing them individually or resetting the device.
// This is tedious, but hopefully updating cassettes will be rare.
// TODO: Avoid manual cleanup.

fn env_flag(key: &'static str) -> bool {
    match std::env::var(key).as_deref() {
        Ok("0") => false,
        Ok("1") => true,
        Ok(s) => panic!("Expected value '0' or '1' but found '{s}' for {key}"),
        Err(_) => false,
    }
}

fn dummy_client() -> Client {
    ClientBuilder::new(Host::parse("localhost").unwrap())
        .build_with_scheme(Scheme::Secure)
        .unwrap()
}

#[derive(Clone, Debug)]
pub struct Library(PathBuf);

impl Library {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self(
            std::env::current_dir()
                .context("no current dir")?
                .join("tests")
                .join("cassette_tests"),
        ))
    }
}

impl Library {
    async fn shelf(&self, client: &Client) -> anyhow::Result<Shelf> {
        let UnrestrictedProperties { prod_nbr, .. } =
            apis::basic_device_info_1::get_all_unrestricted_properties()
                .send(client)
                .await?
                .property_list;

        Ok(Shelf(self.0.join(prod_nbr)))
    }

    fn shelves(&self) -> anyhow::Result<Vec<Shelf>> {
        let mut shelves = Vec::new();
        for entry in fs::read_dir(self.0.as_path())? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                shelves.push(Shelf(entry.path()));
            }
        }
        Ok(shelves)
    }
}

struct Shelf(PathBuf);

impl Shelf {
    fn cassette(&self, name: &'static str, mode: Mode) -> Cassette {
        Cassette::new(self.0.join(name), mode)
    }
}

struct Setup {
    client: Client,
    cassettes: Vec<Cassette>,
}

async fn setup(test_name: &'static str) -> Setup {
    let _ = env_logger::Builder::new()
        .filter_level(LevelFilter::Trace)
        .parse_default_env()
        .is_test(true)
        .try_init();

    let library = Library::new().unwrap();

    match env_flag("UPDATE_CASSETTES") {
        true => {
            let client = ClientBuilder::from_dut()
                .unwrap()
                .unwrap()
                .with_inner(|b| b.danger_accept_invalid_certs(true))
                .build_with_automatic_scheme()
                .await
                .unwrap();
            let shelf = library.shelf(&client).await.unwrap();

            let cassette = shelf.cassette(test_name, Mode::Write);
            cassette.clear().unwrap();
            Setup {
                client,
                cassettes: vec![cassette],
            }
        }
        false => {
            let client = dummy_client();
            let shelves = library.shelves().unwrap();
            let cassettes = shelves
                .into_iter()
                .map(|shelf| shelf.cassette(test_name, Mode::Read))
                .collect();
            Setup { client, cassettes }
        }
    }
}

#[tokio::test]
async fn device_configuration_item_does_not_exist() {
    let Setup { client, cassettes } = setup("device_configuration_item_does_not_exist").await;

    let id = DestinationId::new("my_destination_id".to_string());

    for cassette in cassettes {
        let mut cassette = Some(cassette);

        let error = DeleteDestinationRequest::new(id.clone())
            .send(&client, cassette.as_mut())
            .await
            .unwrap_err();

        let http::Error::Service(error) = error else {
            panic!("Expected Service error but got {error:?}");
        };

        assert_eq!(error.kind().unwrap(), ErrorKind::NotFound);
    }
}

#[tokio::test]
async fn device_configuration_validation_error() {
    let Setup { client, cassettes } = setup("device_configuration_validation_error").await;

    let id = DestinationId::new("my_destination_id".to_string());

    // Test
    for cassette in cassettes {
        let mut cassette = Some(cassette);

        let error = CreateDestinationRequest::azure(
            id.clone(),
            AzureDestination::new(
                "my-container".to_string(),
                "".to_string(),
                Url::parse("https://s3.eu-north-1.amazonaws.com").unwrap(),
            ),
        )
        .send(&client, cassette.as_mut())
        .await
        .unwrap_err();

        let http::Error::Service(error) = error else {
            panic!("Expected Service error but got {error:?}");
        };

        assert_eq!(error.kind().unwrap(), ErrorKind::ValidationError);
    }
}

#[tokio::test]
async fn device_configuration_item_already_exists() {
    let Setup { client, cassettes } = setup("device_configuration_item_already_exists").await;

    let id = DestinationId::new("my_destination_id".to_string());

    // Test
    for cassette in cassettes {
        let mut cassette = Some(cassette);

        let DestinationData { .. } = CreateDestinationRequest::azure(
            id.clone(),
            AzureDestination::new(
                "my-container".to_string(),
                "my-sas".to_string(),
                Url::parse("https://s3.eu-north-1.amazonaws.com").unwrap(),
            ),
        )
        .send(&client, cassette.as_mut())
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
        .send(&client, cassette.as_mut())
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
            .send(&client, cassette.as_mut())
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn recording_group_2_crud() {
    let Setup { client, cassettes } = setup("recording_group_2_crud").await;

    let destination_id = DestinationId::new("rg2_destination_id".to_string());

    // Commented out to avoid waiting for failed requests when replaying.
    // Left in for easier cleanup.

    // let all = ListRecordingGroupsRequest::new()
    //     .send(&client, None)
    //     .await
    //     .unwrap();
    // for g in all {
    //     let _ = DeleteRecordingGroupRequest::new(g.id)
    //         .send(&client, None)
    //         .await;
    // }
    // let _ = DeleteDestinationRequest::new(destination_id.clone())
    //     .send(&client, None)
    //     .await;

    for cassette in cassettes {
        let mut cassette = Some(cassette);

        // Setup: Create a remote object storage destination
        let DestinationData { .. } = CreateDestinationRequest::azure(
            destination_id.clone(),
            AzureDestination::new(
                "my-container".to_string(),
                "my-sas".to_string(),
                Url::parse("https://s3.eu-north-1.amazonaws.com").unwrap(),
            ),
        )
        .send(&client, cassette.as_mut())
        .await
        .unwrap();

        // Create
        let created = CreateRecordingGroupRequest::remote_object_storage(destination_id.clone())
            .send(&client, cassette.as_mut())
            .await
            .unwrap();

        // List
        let all = ListRecordingGroupsRequest::new()
            .send(&client, cassette.as_mut())
            .await
            .unwrap();
        assert!(all.iter().any(|g| g.id == created.id));

        // Delete
        let () = DeleteRecordingGroupRequest::new(created.id.clone())
            .send(&client, cassette.as_mut())
            .await
            .unwrap();

        // Verify deletion
        let all = ListRecordingGroupsRequest::new()
            .send(&client, cassette.as_mut())
            .await
            .unwrap();
        assert!(!all.iter().any(|g| g.id == created.id));

        // Cleanup: Delete the remote object storage destination
        DeleteDestinationRequest::new(destination_id.clone())
            .send(&client, cassette.as_mut())
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn remote_object_storage_1_beta_crud() {
    let Setup { client, cassettes } = setup("remote_object_storage_1_beta_crud").await;

    let id = DestinationId::new("my_destination_id".to_string());

    // Commented out to avoid waiting for failed requests when replaying.
    // Left in for easier cleanup.

    // let _ = DeleteDestinationRequest::new(id.clone())
    //     .send(&client, None)
    //     .await;

    for cassette in cassettes {
        let mut cassette = Some(cassette);

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
        .send(&client, cassette.as_mut())
        .await
        .unwrap();
        assert_eq!(&created.id, &id);
        assert!(created.azure.is_some());

        // List
        let all = ListDestinationsRequest::new()
            .send(&client, cassette.as_mut())
            .await
            .unwrap();
        assert!(all.iter().any(|d| d.id == created.id));

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
        .send(&client, cassette.as_mut())
        .await
        .unwrap();
        // The effect of this update cannot be observed since the sas is redacted.

        let updated_description = format!("{}-updated", created.description.unwrap());
        let () =
            UpdateDestinationRequest::description(created.id.clone(), updated_description.clone())
                .send(&client, cassette.as_mut())
                .await
                .unwrap();
        let all = ListDestinationsRequest::new()
            .send(&client, cassette.as_mut())
            .await
            .unwrap();
        let updated = all.into_iter().find(|d| d.id == created.id).unwrap();
        assert_eq!(updated.description.unwrap(), updated_description);

        // Delete
        let () = DeleteDestinationRequest::new(created.id.clone())
            .send(&client, cassette.as_mut())
            .await
            .unwrap();

        // Verify deletion
        let all = ListDestinationsRequest::new()
            .send(&client, cassette.as_mut())
            .await
            .unwrap();
        assert!(!all.iter().any(|d| d.id == created.id));
    }
}

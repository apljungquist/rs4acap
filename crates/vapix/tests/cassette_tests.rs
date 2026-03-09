use std::{fs, future::Future, path::PathBuf, pin::Pin};

use anyhow::Context;
use libtest_mimic::{Arguments, Trial};
use log::LevelFilter;
use rs4a_vapix::{
    apis,
    basic_device_info_1::UnrestrictedProperties,
    cassette::{Cassette, Mode},
    http,
    json_rpc_http::JsonRpcHttp,
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

// When a test fails, it may leave resources intact that will cause future runs to fail.
// This must be cleaned up manually by either removing them individually or resetting the device.
// This is tedious, but hopefully updating cassettes will be rare.
// TODO: Avoid manual cleanup.

fn env_flag(key: &str) -> bool {
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
    fn name(&self) -> &str {
        self.0.file_name().unwrap().to_str().unwrap()
    }

    fn cassette(&self, name: &str, mode: Mode) -> Cassette {
        Cassette::new(self.0.join(name), mode)
    }
}

type TestFn = fn(Client, Cassette) -> Pin<Box<dyn Future<Output = ()> + Send>>;

const TESTS: &[(&str, TestFn)] = &[
    (
        "device_configuration_item_does_not_exist",
        |client, cassette| Box::pin(device_configuration_item_does_not_exist(client, cassette)),
    ),
    (
        "device_configuration_validation_error",
        |client, cassette| Box::pin(device_configuration_validation_error(client, cassette)),
    ),
    (
        "device_configuration_item_already_exists",
        |client, cassette| Box::pin(device_configuration_item_already_exists(client, cassette)),
    ),
    ("remote_object_storage_1_beta_crud", |client, cassette| {
        Box::pin(remote_object_storage_1_beta_crud(client, cassette))
    }),
];

fn record_trials(library: &Library) -> Vec<Trial> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let client = rt.block_on(async {
        ClientBuilder::from_dut()
            .unwrap()
            .unwrap()
            .with_inner(|b| b.danger_accept_invalid_certs(true))
            .build_with_automatic_scheme()
            .await
            .unwrap()
    });
    let shelf = rt.block_on(library.shelf(&client)).unwrap();

    TESTS
        .iter()
        .map(|&(test_name, test_fn)| {
            let cassette = shelf.cassette(test_name, Mode::Write);
            cassette.clear().unwrap();
            let client = client.clone();
            Trial::test(test_name, move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(test_fn(client, cassette));
                Ok(())
            })
        })
        .collect()
}

fn playback_trials(library: &Library) -> Vec<Trial> {
    let client = dummy_client();
    let shelves = library.shelves().unwrap();

    shelves
        .into_iter()
        .flat_map(|shelf| {
            let shelf_name = shelf.name().to_string();
            let client = client.clone();
            TESTS.iter().map(move |&(test_name, test_fn)| {
                let trial_name = format!("{test_name}::{shelf_name}");
                let cassette_dir = shelf.0.join(test_name);
                let has_cassette = cassette_dir.exists();
                let client = client.clone();
                let mut trial = Trial::test(trial_name, move || {
                    let cassette = Cassette::new(cassette_dir, Mode::Read);
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    rt.block_on(test_fn(client, cassette));
                    Ok(())
                });
                if !has_cassette {
                    trial = trial.with_ignored_flag(true);
                }
                trial
            })
        })
        .collect()
}

fn main() {
    let _ = env_logger::Builder::new()
        .filter_level(LevelFilter::Warn)
        .parse_default_env()
        .is_test(true)
        .try_init();

    let mut args = Arguments::from_args();
    let library = Library::new().unwrap();

    let trials = match env_flag("UPDATE_CASSETTES") {
        true => record_trials(&library),
        false => playback_trials(&library),
    };

    if args.test_threads.is_none() {
        println!("Running tests in single-threaded mode");
        args.test_threads = Some(1);
    }

    libtest_mimic::run(&args, trials).exit();
}

async fn device_configuration_item_does_not_exist(client: Client, cassette: Cassette) {
    let mut cassette = Some(cassette);

    let id = DestinationId::new("my_destination_id".to_string());

    let error = DeleteDestinationRequest::new(id.clone())
        .send(&client, cassette.as_mut())
        .await
        .unwrap_err();

    let http::Error::Service(error) = error else {
        panic!("Expected Service error but got {error:?}");
    };

    assert_eq!(error.kind().unwrap(), ErrorKind::NotFound);
}

async fn device_configuration_validation_error(client: Client, cassette: Cassette) {
    let mut cassette = Some(cassette);

    let id = DestinationId::new("my_destination_id".to_string());

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

async fn device_configuration_item_already_exists(client: Client, cassette: Cassette) {
    let mut cassette = Some(cassette);

    let id = DestinationId::new("my_destination_id".to_string());

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

async fn remote_object_storage_1_beta_crud(client: Client, cassette: Cassette) {
    let mut cassette = Some(cassette);

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
    let () = UpdateDestinationRequest::description(created.id.clone(), updated_description.clone())
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

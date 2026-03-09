use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    future::Future,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    pin::Pin,
};

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
use serde::{Deserialize, Serialize};
use url::{Host, Url};

// Responses may vary due to time, initial state of the device, test order, etc.
// Manually determining when cassettes should be updated is tedious and error prone.
// Furthermore, if cassettes are addressed by their content, it cache hits will suffer.
// TODO: Automatically deal with non-reproducible responses.

// When a test fails, it may leave resources intact that will cause future runs to fail.
// This must be cleaned up manually by either removing them individually or resetting the device.
// This is tedious, but hopefully updating cassettes will be rare.
// TODO: Avoid manual cleanup.

#[derive(Clone, Debug)]
struct Prelude {
    props: UnrestrictedProperties,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DeviceInfo {
    prod_nbr: String,
    version: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Manifest {
    #[serde(default)]
    groups: BTreeMap<String, Vec<String>>,
    devices: BTreeMap<String, DeviceInfo>,
    cassettes: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

impl Manifest {
    fn load(path: &Path) -> anyhow::Result<Self> {
        match fs::read_to_string(path) {
            Ok(content) => Ok(serde_json::from_str(&content)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    fn save(&self, path: &Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn resolve_label(&self, test_name: &str, hash: &str) -> String {
        let devices = match self.cassettes.get(test_name).and_then(|t| t.get(hash)) {
            Some(devices) => devices.iter().collect::<BTreeSet<_>>(),
            None => return hash.to_string(),
        };

        let mut matched = None;
        for (label, group_devices) in &self.groups {
            let group_set = group_devices.iter().collect::<BTreeSet<_>>();
            if devices == group_set {
                if let Some(prev) = matched {
                    panic!(
                        "Ambiguous group match for {test_name}::{hash}: \
                         both '{prev}' and '{label}' match the same device set"
                    );
                }
                matched = Some(label.as_str());
            }
        }

        matched.unwrap_or(hash).to_string()
    }
}

fn content_hash_of_dir(dir: &Path) -> anyhow::Result<String> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());

    // Use DefaultHasher to match the hashing approach in cassette.rs.
    let mut hasher = DefaultHasher::new();
    for entry in &entries {
        entry.file_name().hash(&mut hasher);
        fs::read(entry.path())?.hash(&mut hasher);
    }

    Ok(format!("{:016x}", hasher.finish()))
}

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

    fn dir(&self) -> &Path {
        &self.0
    }

    fn staging_dir(&self, test_name: &str) -> PathBuf {
        self.0.join(test_name).join(".staging")
    }

    fn clean_staging(&self, test_name: &str) {
        let staging = self.staging_dir(test_name);
        let _ = fs::remove_dir_all(&staging);
    }

    fn cassettes_for_test(&self, test_name: &str) -> Vec<PathBuf> {
        let test_dir = self.0.join(test_name);
        let mut dirs = Vec::new();
        if let Ok(entries) = fs::read_dir(&test_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if !name_str.starts_with('.') {
                        dirs.push(path);
                    }
                }
            }
        }
        dirs.sort();
        dirs
    }
}

fn finalize_recording(
    library_dir: &Path,
    test_name: &str,
    staging_dir: &Path,
    device_key: &str,
    device_info: &DeviceInfo,
) -> anyhow::Result<()> {
    // If staging dir is empty or doesn't exist, the test was skipped
    let is_empty = match fs::read_dir(staging_dir) {
        Ok(mut entries) => entries.next().is_none(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(e) => return Err(e.into()),
    };
    if is_empty {
        let _ = fs::remove_dir_all(staging_dir);
        return Ok(());
    }

    let hash = content_hash_of_dir(staging_dir)?;
    let dest_dir = library_dir.join(test_name).join(&hash);

    if dest_dir.exists() {
        // Duplicate cassette - remove staging
        fs::remove_dir_all(staging_dir)?;
    } else {
        fs::rename(staging_dir, &dest_dir)?;
    }

    // Update manifest
    let manifest_path = library_dir.join("manifest.json");
    let mut manifest = Manifest::load(&manifest_path)?;

    manifest
        .devices
        .insert(device_key.to_string(), device_info.clone());

    let test_entry = manifest.cassettes.entry(test_name.to_string()).or_default();

    // Remove device from any old hash entry for this test
    for devices in test_entry.values_mut() {
        devices.retain(|d| d != device_key);
    }
    // Remove empty entries
    test_entry.retain(|_, devices| !devices.is_empty());

    // Add device to new hash entry
    let hash_entry = test_entry.entry(hash).or_default();
    if !hash_entry.contains(&device_key.to_string()) {
        hash_entry.push(device_key.to_string());
        hash_entry.sort();
    }

    manifest.save(&manifest_path)?;
    Ok(())
}

type TestFn = fn(Client, Cassette, Option<Prelude>) -> Pin<Box<dyn Future<Output = ()> + Send>>;

const TESTS: &[(&str, TestFn)] = &[
    (
        "device_configuration_item_does_not_exist",
        |client, cassette, prelude| {
            Box::pin(device_configuration_item_does_not_exist(
                client, cassette, prelude,
            ))
        },
    ),
    (
        "device_configuration_validation_error",
        |client, cassette, prelude| {
            Box::pin(device_configuration_validation_error(
                client, cassette, prelude,
            ))
        },
    ),
    (
        "device_configuration_item_already_exists",
        |client, cassette, prelude| {
            Box::pin(device_configuration_item_already_exists(
                client, cassette, prelude,
            ))
        },
    ),
    (
        "remote_object_storage_1_beta_crud",
        |client, cassette, prelude| {
            Box::pin(remote_object_storage_1_beta_crud(client, cassette, prelude))
        },
    ),
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

    let prelude = rt.block_on(async {
        let props = apis::basic_device_info_1::get_all_unrestricted_properties()
            .send(&client)
            .await
            .unwrap()
            .property_list;
        Prelude { props }
    });
    let device_key = format!("{}@{}", prelude.props.prod_nbr, prelude.props.version);
    let device_info = DeviceInfo {
        prod_nbr: prelude.props.prod_nbr.clone(),
        version: prelude.props.version.clone(),
    };

    TESTS
        .iter()
        .map(|&(test_name, test_fn)| {
            library.clean_staging(test_name);
            let staging_dir = library.staging_dir(test_name);
            let cassette = Cassette::new(staging_dir.clone(), Mode::Write);
            cassette.clear().unwrap();
            let client = client.clone();
            let prelude = prelude.clone();
            let library_dir = library.dir().to_path_buf();
            let device_key = device_key.clone();
            let device_info = device_info.clone();

            Trial::test(test_name, move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(test_fn(client, cassette, Some(prelude)));

                finalize_recording(
                    &library_dir,
                    test_name,
                    &staging_dir,
                    &device_key,
                    &device_info,
                )?;
                Ok(())
            })
        })
        .collect()
}

fn playback_trials(library: &Library) -> Vec<Trial> {
    let client = dummy_client();
    let manifest_path = library.dir().join("manifest.json");
    let manifest = Manifest::load(&manifest_path).unwrap();

    TESTS
        .iter()
        .flat_map(|&(test_name, test_fn)| {
            let cassette_dirs = library.cassettes_for_test(test_name);
            let client = client.clone();
            let manifest = &manifest;
            cassette_dirs.into_iter().map(move |cassette_dir| {
                let hash = cassette_dir
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
                let label = manifest.resolve_label(test_name, &hash);
                let trial_name = format!("{test_name}::{label}");
                let client = client.clone();
                Trial::test(trial_name, move || {
                    let cassette = Cassette::new(cassette_dir, Mode::Read);
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    rt.block_on(test_fn(client, cassette, None));
                    Ok(())
                })
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

async fn device_configuration_item_does_not_exist(
    client: Client,
    cassette: Cassette,
    _prelude: Option<Prelude>,
) {
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

async fn device_configuration_validation_error(
    client: Client,
    cassette: Cassette,
    _prelude: Option<Prelude>,
) {
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

async fn device_configuration_item_already_exists(
    client: Client,
    cassette: Cassette,
    _prelude: Option<Prelude>,
) {
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

async fn remote_object_storage_1_beta_crud(
    client: Client,
    cassette: Cassette,
    _prelude: Option<Prelude>,
) {
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

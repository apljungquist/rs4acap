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
use regex::Regex;
use rs4a_vapix::{
    apis,
    apis::basic_device_info_1,
    basic_device_info_1::{ProductType, UnrestrictedProperties},
    cassette::{Cassette, Mode},
    http,
    json_rpc_http::{JsonRpcHttp, JsonRpcHttpLossless},
    parameter_management::{ImageResolution, ListRequest},
    remote_object_storage_1_beta::{
        AzureDestination, CreateDestinationRequest, DeleteDestinationRequest, DestinationData,
        DestinationId, ListDestinationsRequest, UpdateDestinationRequest,
    },
    rest::ErrorKind,
    rest_http2::RestHttp2,
    siren_and_light_2_alpha::{
        GetMaintenanceModeRequest, StartMaintenanceModeRequest, StopMaintenanceModeRequest,
    },
    Client, ClientBuilder, Scheme,
};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use url::{Host, Url};
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

#[derive(Clone, Debug)]
struct Prelude {
    props: UnrestrictedProperties,
}

impl Prelude {
    pub(crate) fn supports_device_config(&self) -> bool {
        self.version_matches(">=11")
    }
}

impl Prelude {
    fn version_matches(&self, req: &str) -> bool {
        let v = Version::parse(self.props.version.as_str()).unwrap();
        let req = VersionReq::parse(req).unwrap();
        req.matches(&v)
    }
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    skipped: BTreeMap<String, Vec<String>>,
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
        let content = serde_json::to_string_pretty(self)? + "\n";
        fs::write(path, content)?;
        Ok(())
    }

    fn resolve_devices_label(&self, devices: &[String], fallback: &str) -> String {
        let device_set = devices.iter().collect::<BTreeSet<_>>();

        let mut matched = None;
        for (label, group_devices) in &self.groups {
            let group_set = group_devices.iter().collect::<BTreeSet<_>>();
            if device_set == group_set {
                if let Some(prev) = matched {
                    panic!(
                        "Ambiguous group match for {fallback}: \
                         both '{prev}' and '{label}' match the same device set"
                    );
                }
                matched = Some(label.as_str());
            }
        }

        if let Some(label) = matched {
            return label.to_string();
        }
        if devices.len() == 1 {
            return devices[0].clone();
        }
        fallback.to_string()
    }

    fn resolve_label(&self, test_name: &str, hash: &str) -> String {
        match self.cassettes.get(test_name).and_then(|t| t.get(hash)) {
            Some(devices) => self.resolve_devices_label(devices, hash),
            None => hash.to_string(),
        }
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

fn normalize_responses(dir: &Path, substitutions: &[(&str, &str)]) -> anyhow::Result<()> {
    let patterns: Vec<(Regex, &str)> = substitutions
        .iter()
        .map(|(pattern, replacement)| {
            let re =
                Regex::new(pattern).with_context(|| format!("Invalid regex pattern: {pattern}"))?;
            Ok((re, *replacement))
        })
        .collect::<anyhow::Result<_>>()?;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if !name.to_string_lossy().ends_with("-response") {
            continue;
        }
        let content = fs::read_to_string(&path)?;
        let mut normalized = content.clone();
        for (re, replacement) in &patterns {
            normalized = re.replace_all(&normalized, *replacement).into_owned();
        }
        if normalized != content {
            fs::write(&path, normalized)?;
        }
    }

    Ok(())
}

fn finalize_recording(
    library_dir: &Path,
    test_name: &str,
    staging_dir: &Path,
    device_key: &str,
    device_info: &DeviceInfo,
    substitutions: &[(&str, &str)],
) -> anyhow::Result<()> {
    // If the staging dir is empty or doesn't exist, the test was skipped
    let is_empty = match fs::read_dir(staging_dir) {
        Ok(mut entries) => entries.next().is_none(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(e) => return Err(e.into()),
    };
    if is_empty {
        let _ = fs::remove_dir_all(staging_dir);

        // Record the skip in the manifest
        let manifest_path = library_dir.join("manifest.json");
        let mut manifest = Manifest::load(&manifest_path)?;
        manifest
            .devices
            .insert(device_key.to_string(), device_info.clone());
        let skipped_entry = manifest.skipped.entry(test_name.to_string()).or_default();
        if !skipped_entry.contains(&device_key.to_string()) {
            skipped_entry.push(device_key.to_string());
            skipped_entry.sort();
        }
        manifest.save(&manifest_path)?;
        return Ok(());
    }

    normalize_responses(staging_dir, substitutions)?;
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

    // Remove the device from skipped since it produced a real cassette
    if let Some(skipped_devices) = manifest.skipped.get_mut(test_name) {
        skipped_devices.retain(|d| d != device_key);
        if skipped_devices.is_empty() {
            manifest.skipped.remove(test_name);
        }
    }

    let test_entry = manifest.cassettes.entry(test_name.to_string()).or_default();

    // Remove the device from any old hash entry for this test
    for devices in test_entry.values_mut() {
        devices.retain(|d| d != device_key);
    }
    // Remove empty entries
    test_entry.retain(|_, devices| !devices.is_empty());

    // Add a device to a new hash entry
    let hash_entry = test_entry.entry(hash).or_default();
    if !hash_entry.contains(&device_key.to_string()) {
        hash_entry.push(device_key.to_string());
        hash_entry.sort();
    }

    manifest.save(&manifest_path)?;
    Ok(())
}

type TestFn = fn(Client, Cassette, Option<Prelude>) -> Pin<Box<dyn Future<Output = ()> + Send>>;
type Substitutions = &'static [(&'static str, &'static str)];
type TestEntry = (&'static str, TestFn, Substitutions);

macro_rules! cassette_tests {
    (@entry $name:ident => [$($sub:expr),* $(,)?]) => {
        (
            stringify!($name),
            (|client, cassette, prelude| {
                Box::pin($name(client, cassette, prelude))
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
    basic_device_info_get_all_properties => [
        (
            r#""SocSerialNumber": "[0-9A-F]{8}-[0-9A-F]{8}-[0-9A-F]{8}-[0-9A-F]{8}""#,
            r#""SocSerialNumber": "00000000-00000000-01234567-89ABCDEF""#,
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
    device_configuration_item_does_not_exist,
    device_configuration_validation_error,
    device_configuration_item_already_exists,
    parameter_management_list_error,
    parameter_management_list_image_resolution,
    remote_object_storage_1_beta_crud,
    siren_and_light_2_alpha_maintenance_mode_not_supported,
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
        .map(|&(test_name, test_fn, substitutions)| {
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
                    substitutions,
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
        .flat_map(|&(test_name, test_fn, _substitutions)| {
            let cassette_dirs = library.cassettes_for_test(test_name);
            let client = client.clone();
            let manifest = &manifest;

            let cassette_trials = cassette_dirs.into_iter().map(move |cassette_dir| {
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
            });

            let skipped_trial = manifest
                .skipped
                .get(test_name)
                .filter(|devices| !devices.is_empty())
                .map(|devices| {
                    let fallback = devices.join("+");
                    let label = manifest.resolve_devices_label(devices, &fallback);
                    let trial_name = format!("{test_name}::{label}");
                    Trial::test(trial_name, || Ok(())).with_ignored_flag(true)
                });

            cassette_trials.chain(skipped_trial)
        })
        .collect()
}

fn cleanup_unreferenced_cassettes(library: &Library) {
    let manifest_path = library.dir().join("manifest.json");
    let manifest = Manifest::load(&manifest_path).unwrap();

    let referenced: BTreeMap<&str, BTreeSet<&str>> = manifest
        .cassettes
        .iter()
        .map(|(test, hashes)| (test.as_str(), hashes.keys().map(|h| h.as_str()).collect()))
        .collect();

    for entry in fs::read_dir(library.dir()).unwrap().flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let test_name = entry.file_name();
        let test_name_str = test_name.to_string_lossy();
        let referenced_hashes = referenced.get(test_name_str.as_ref());

        for sub_entry in fs::read_dir(&path).unwrap().flatten() {
            let sub_path = sub_entry.path();
            if !sub_path.is_dir() {
                continue;
            }
            let name = sub_entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') {
                continue;
            }
            let is_referenced =
                referenced_hashes.map_or(false, |hashes| hashes.contains(name_str.as_ref()));
            if !is_referenced {
                println!("Removing unreferenced cassette: {sub_path:?}");
                fs::remove_dir_all(&sub_path).unwrap();
            }
        }
    }
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
        cleanup_unreferenced_cassettes(&library);
    }
    conclusion.exit();
}

async fn basic_device_info_get_all_properties(
    client: Client,
    cassette: Cassette,
    _: Option<Prelude>,
) {
    let mut cassette = Some(cassette);
    let property_list = basic_device_info_1::get_all_properties()
        .send_lossless(&client, cassette.as_mut())
        .await
        .unwrap()
        .property_list;

    assert!(property_list.restricted.parse_soc_serial_number().is_ok())
}

async fn basic_device_info_get_all_unrestricted_properties(
    client: Client,
    cassette: Cassette,
    _: Option<Prelude>,
) {
    let mut cassette = Some(cassette);
    let property_list = basic_device_info_1::get_all_unrestricted_properties()
        .send_lossless(&client, cassette.as_mut())
        .await
        .unwrap()
        .property_list;

    property_list.parse_product_type().unwrap();
}

async fn device_configuration_item_does_not_exist(
    client: Client,
    cassette: Cassette,
    prelude: Option<Prelude>,
) {
    if let Some(prelude) = prelude {
        if !prelude.supports_device_config() {
            return;
        }
    }

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
    prelude: Option<Prelude>,
) {
    if let Some(prelude) = prelude {
        if !prelude.supports_device_config() {
            return;
        }
    }

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
    prelude: Option<Prelude>,
) {
    if let Some(prelude) = prelude {
        if !prelude.supports_device_config() {
            return;
        }
    }

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

async fn parameter_management_list_error(
    client: Client,
    cassette: Cassette,
    prelude: Option<Prelude>,
) {
    if let Some(prelude) = prelude {
        match prelude.props.parse_product_type().unwrap() {
            ProductType::BoxCamera => return,
            ProductType::DomeCamera => return,
            ProductType::NetworkCamera => return,
            ProductType::NetworkStrobeSpeaker => {}
            ProductType::Radar => return,
            ProductType::PeopleCounter3D => return,
            _ => {}
        }
    }

    let mut cassette = Some(cassette);
    let error = ListRequest::new::<ImageResolution>()
        .send(&client, cassette.as_mut())
        .await
        .unwrap_err();

    // TODO: Parse the error code and message
    assert!(error.to_string().contains("-1"));
}

async fn parameter_management_list_image_resolution(
    client: Client,
    cassette: Cassette,
    prelude: Option<Prelude>,
) {
    if let Some(prelude) = prelude {
        if matches!(
            prelude.props.parse_product_type().unwrap(),
            ProductType::NetworkStrobeSpeaker
        ) {
            return;
        }
    }

    let mut cassette = Some(cassette);
    let params = ListRequest::new::<ImageResolution>()
        .send(&client, cassette.as_mut())
        .await
        .unwrap();
    let resolutions = params.parse::<ImageResolution>().unwrap().unwrap();
    assert!(!resolutions.is_empty());
}

async fn remote_object_storage_1_beta_crud(
    client: Client,
    cassette: Cassette,
    prelude: Option<Prelude>,
) {
    if let Some(prelude) = prelude {
        if !prelude.supports_device_config() {
            return;
        }
    }

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
    assert_eq!(all.len(), 1);

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

async fn siren_and_light_2_alpha_maintenance_mode_not_supported(
    client: Client,
    cassette: Cassette,
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
        if ["M1075-L", "D2210-VE"].contains(&prelude.props.prod_nbr.as_str()) {
            return;
        }
    }

    let mut cassette = Some(cassette);

    GetMaintenanceModeRequest::new()
        .send(&client, cassette.as_mut())
        .await
        .unwrap();

    let error = StopMaintenanceModeRequest::new()
        .send(&client, cassette.as_mut())
        .await
        .unwrap_err();

    let http::Error::Service(error) = error else {
        panic!("Expected Service error but got {error:?}");
    };

    assert_eq!(error.kind(), Some(ErrorKind::InternalError));

    let error = StartMaintenanceModeRequest::new()
        .send(&client, cassette.as_mut())
        .await
        .unwrap_err();

    let http::Error::Service(error) = error else {
        panic!("Expected Service error but got {error:?}");
    };

    assert_eq!(error.kind(), Some(ErrorKind::InternalError));
}

async fn system_ready_1_system_ready(
    client: Client,
    cassette: Cassette,
    _prelude: Option<Prelude>,
) {
    let mut cassette = Some(cassette);
    let data = apis::system_ready_1::system_ready()
        .send_lossless(&client, cassette.as_mut())
        .await
        .unwrap();
    assert!(data.systemready);
}

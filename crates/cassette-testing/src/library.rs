use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    fs::create_dir_all,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::cassette::Cassette;

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

    /// Load all cassettes from disk, grouped by test name and label.
    /// `None` values represent tests that were skipped during recording.
    pub fn cassettes(&self) -> anyhow::Result<HashMap<String, HashMap<String, Option<Cassette>>>> {
        let manifest = Manifest::load(&self.0.join("manifest.json"))?;
        let mut result: HashMap<String, HashMap<String, Option<Cassette>>> = HashMap::new();

        for (test_name, hash_map) in &manifest.cassettes {
            for (hash, devices) in hash_map {
                let cassette_dir = self.0.join(test_name).join(hash);
                if !cassette_dir.is_dir() {
                    // TODO: Remove missing cassettes from the manifest.
                    continue;
                }
                let label = manifest.resolve_devices_label(devices);
                let cassette = load_cassette(&cassette_dir)?;
                result
                    .entry(test_name.clone())
                    .or_default()
                    .insert(label, Some(cassette));
            }
        }

        for (test_name, devices) in &manifest.skipped {
            if devices.is_empty() {
                continue;
            }
            let label = manifest.resolve_devices_label(devices);
            result
                .entry(test_name.clone())
                .or_default()
                .insert(label, None);
        }

        Ok(result)
    }

    /// Write a recorded cassette to disk, applying substitutions and updating the manifest.
    pub fn write(
        &self,
        test_name: &str,
        device_info: &DeviceInfo,
        cassette: &Cassette,
    ) -> anyhow::Result<()> {
        let device_key = device_info.device_key();

        if cassette.is_empty() {
            let manifest_path = self.0.join("manifest.json");
            let mut manifest = Manifest::load(&manifest_path)?;
            manifest
                .devices
                .insert(device_key.clone(), device_info.clone());
            let skipped_entry = manifest.skipped.entry(test_name.to_string()).or_default();
            if !skipped_entry.contains(&device_key) {
                skipped_entry.push(device_key);
                skipped_entry.sort();
            }
            manifest.save(&manifest_path)?;
            return Ok(());
        }

        let tracks = cassette.normalized_tracks()?;

        // Build file contents and compute content hash in memory.
        // TODO: Simplify filenames to `{i:>03}-request` / `{i:>03}-response` once all
        // cassettes are re-recorded; the checksum in the filename is vestigial since
        // playback is now sequential.
        let mut files: Vec<(String, &str)> = Vec::new();
        for (i, (checksum, req, resp)) in tracks.iter().enumerate() {
            files.push((format!("{i:>03}-{checksum:016x}-request"), *req));
            files.push((format!("{i:>03}-{checksum:016x}-response"), resp.as_str()));
        }
        files.sort_by(|a, b| a.0.cmp(&b.0));

        // Compute content hash matching the on-disk layout: hash each filename as
        // OsString and each file's content as bytes, matching the old content_hash_of_dir.
        // TODO: Use a stable hashing algorithm; DefaultHasher is not guaranteed to be
        // stable across Rust versions, invalidating VCS-tracked cassettes.
        let mut hasher = DefaultHasher::new();
        for (name, content) in &files {
            std::ffi::OsString::from(name).hash(&mut hasher);
            content.as_bytes().hash(&mut hasher);
        }
        let hash = format!("{:016x}", hasher.finish());

        let dest_dir = self.0.join(test_name).join(&hash);
        if !dest_dir.exists() {
            create_dir_all(&dest_dir)?;
            for (name, content) in &files {
                fs::write(dest_dir.join(name), content)?;
            }
        }

        // Update manifest
        let manifest_path = self.0.join("manifest.json");
        let mut manifest = Manifest::load(&manifest_path)?;

        manifest
            .devices
            .insert(device_key.clone(), device_info.clone());

        if let Some(skipped_devices) = manifest.skipped.get_mut(test_name) {
            skipped_devices.retain(|d| d != &device_key);
            if skipped_devices.is_empty() {
                manifest.skipped.remove(test_name);
            }
        }

        let test_entry = manifest.cassettes.entry(test_name.to_string()).or_default();
        for devices in test_entry.values_mut() {
            devices.retain(|d| d != &device_key);
        }
        test_entry.retain(|_, devices| !devices.is_empty());

        let hash_entry = test_entry.entry(hash).or_default();
        if !hash_entry.contains(&device_key) {
            hash_entry.push(device_key);
            hash_entry.sort();
        }

        manifest.save(&manifest_path)?;
        Ok(())
    }

    pub fn cleanup_unreferenced(&self) -> anyhow::Result<()> {
        let manifest = Manifest::load(&self.0.join("manifest.json"))?;

        let referenced: BTreeMap<&str, BTreeSet<&str>> = manifest
            .cassettes
            .iter()
            .map(|(test, hashes)| (test.as_str(), hashes.keys().map(|h| h.as_str()).collect()))
            .collect();

        for entry in fs::read_dir(&self.0)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let test_name = entry.file_name();
            let test_name_str = test_name.to_string_lossy();
            let referenced_hashes = referenced.get(test_name_str.as_ref());

            for sub_entry in fs::read_dir(&path)? {
                let sub_entry = sub_entry?;
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
                    referenced_hashes.is_some_and(|hashes| hashes.contains(name_str.as_ref()));
                if !is_referenced {
                    log::info!("Removing unreferenced cassette: {sub_path:?}");
                    fs::remove_dir_all(&sub_path)?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub prod_nbr: String,
    pub version: String,
}

impl DeviceInfo {
    pub fn device_key(&self) -> String {
        format!("{}@{}", self.prod_nbr, self.version)
    }
}

/// Parse the checksum from a filename like `000-2615ae97f47c1679-request`.
fn parse_checksum(filename: &str) -> anyhow::Result<u64> {
    let hex = filename
        .split('-')
        .nth(1)
        .context("missing checksum in filename")?;
    u64::from_str_radix(hex, 16).context("invalid checksum hex")
}

fn load_cassette(dir: &Path) -> anyhow::Result<Cassette> {
    let mut request_files: Vec<_> = Vec::new();
    let mut response_files: Vec<_> = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();
        if name_str.ends_with("-request") {
            request_files.push((name_str, entry.path()));
        } else if name_str.ends_with("-response") {
            response_files.push((name_str, entry.path()));
        }
    }

    request_files.sort_by(|a, b| a.0.cmp(&b.0));
    response_files.sort_by(|a, b| a.0.cmp(&b.0));

    anyhow::ensure!(
        request_files.len() == response_files.len(),
        "Mismatched request/response count in {dir:?}: {} requests, {} responses",
        request_files.len(),
        response_files.len()
    );

    let tracks = request_files
        .iter()
        .zip(&response_files)
        .map(|((req_name, req_path), (_, resp_path))| {
            let checksum = parse_checksum(req_name)?;
            let request =
                fs::read_to_string(req_path).with_context(|| format!("reading {req_path:?}"))?;
            let response =
                fs::read_to_string(resp_path).with_context(|| format!("reading {resp_path:?}"))?;
            Ok((checksum, request, response))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(Cassette::loaded(tracks))
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

    fn resolve_devices_label(&self, devices: &[String]) -> String {
        let fallback = devices.join("+");
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
        // TODO: Reconsider the correct behavior when there are 0 or more than 1 devices.
        if let [d] = devices {
            return d.clone();
        }
        fallback
    }
}

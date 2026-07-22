//! Resolution and validation of manifests against their JSON schema.
use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context};
use log::debug;
use serde_json::Value;

use crate::json_ext::{MapExt, ValueExt};

/// Where to find the schema used to validate a manifest.
#[derive(Clone, Debug, Default)]
pub enum SchemaSource {
    /// Validate against the schema at this path, ignoring the manifest's `schemaVersion`.
    File(PathBuf),
    /// Resolve a schema by the manifest's `schemaVersion` from an installed SDK.
    ///
    /// The path should be that of the `schemas` dir.
    Resolve(PathBuf),
    /// Do not validate the manifest.
    #[default]
    None,
}

/// Validate `manifest` against the schema indicated by `source`.
pub fn validate(manifest: &Value, source: &SchemaSource) -> anyhow::Result<()> {
    let schema = match source {
        SchemaSource::None => {
            debug!("Skipping manifest validation");
            return Ok(());
        }
        SchemaSource::File(path) => {
            debug!("Validating manifest against schema {path:?}");
            read_schema(path)?
        }
        SchemaSource::Resolve(schemas_dir) => resolve(schemas_dir, manifest)?,
    };

    let validator = jsonschema::options()
        .should_validate_formats(false)
        .build(&schema)
        .map_err(|e| anyhow!("Invalid schema: {e}"))?;
    let errors: Vec<String> = validator
        .iter_errors(manifest)
        .map(|e| {
            format!(
                "- {e} (at {})",
                match e.instance_path.as_str() {
                    "" => "schema root",
                    s => s,
                }
            )
        })
        .collect();
    if !errors.is_empty() {
        bail!("Manifest failed schema validation:\n{}", errors.join("\n"));
    }
    Ok(())
}

fn read_schema(path: &Path) -> anyhow::Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("Reading schema {path:?}"))?;
    serde_json::from_str(&text).with_context(|| format!("Parsing schema {path:?}"))
}

/// Resolve a schema by the manifest's `schemaVersion` from the installed SDK.
fn resolve(schemas_dir: &Path, manifest: &Value) -> anyhow::Result<Value> {
    resolve_in_dir(manifest, schemas_dir)
}

fn resolve_in_dir(manifest: &Value, sdk_dir: &Path) -> anyhow::Result<Value> {
    let version = manifest.try_to_object()?.try_get_str("schemaVersion")?;

    // This long error message is essentially a help text for a user of the binary,
    // but this is the library and this error message may not suit other dependants.
    // TODO: Implement structured error handling
    match find_in_dir(sdk_dir, version) {
        Some(path) => {
            debug!("Validating manifest against SDK schema {path:?}");
            read_schema(&path)
        }
        None => bail!(
            "No schema for schemaVersion {version:?} found under {sdk_dir:?}. Supply a schema \
             with `SchemaSource::File`, point `SchemaSource::Resolve` at an SDK installation that \
             provides it, or skip validation with `SchemaSource::None`."
        ),
    }
}

/// Find the schema file matching `version` under `dir`, if any.
///
/// Returns the lexically first match for determinism when a partial version matches several files.
fn find_in_dir(dir: &Path, version: &str) -> Option<PathBuf> {
    let mut matches = Vec::new();
    collect_json(dir, &mut matches);
    matches
        .into_iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| version_matches_file(version, n))
        })
        .min()
}

fn collect_json(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_json(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
            out.push(path);
        }
    }
}

/// Whether a schema file named `file_name` should be used for `version`.
fn version_matches_file(version: &str, file_name: &str) -> bool {
    file_name.ends_with(&format!("v{version}.json"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn none_skips_validation() {
        // An obviously invalid manifest passes when validation is disabled.
        validate(&json!({}), &SchemaSource::None).unwrap();
    }

    #[test]
    fn version_matches_only_its_exact_schema_file() {
        let file = |v: &str| format!("application-manifest-schema-v{v}.json");

        assert!(version_matches_file("1.3", &file("1.3")));
        assert!(version_matches_file("1.7.5", &file("1.7.5")));

        // A two-segment version must not match its three-segment successors.
        assert!(!version_matches_file("1.3", &file("1.3.1")));
        // A version must not match a longer version that merely contains it.
        assert!(!version_matches_file("1.1", &file("1.10.0")));
        assert!(!version_matches_file("1.1", &file("1.11.0")));
        assert!(!version_matches_file("1.7", &file("1.7.5")));
    }
}

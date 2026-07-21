use anyhow::bail;
use log::debug;
use serde::Serialize;
use serde_json::{ser::PrettyFormatter, Map, Serializer, Value};

use crate::{
    json_ext,
    json_ext::{MapExt, ValueExt},
    Architecture,
};

#[derive(Debug)]
pub(crate) struct Manifest(Value);

impl Manifest {
    pub(crate) fn new(manifest: Value, architecture: Architecture) -> anyhow::Result<Self> {
        let mut manifest = Self(manifest);
        let mut schema_version = manifest
            .as_object()?
            .try_get_str("schemaVersion")?
            .to_string();

        // Make it valid semver
        for _ in 0..2usize.saturating_sub(schema_version.chars().filter(|&c| c == '.').count()) {
            schema_version.push_str(".0");
        }
        let schema_version = semver::Version::parse(&schema_version)?;
        if schema_version >= semver::Version::new(1, 3, 0) {
            let setup = manifest.try_find_setup_mut()?;
            if let Some(a) = setup.get("architecture") {
                if a != "all" && a != architecture.as_str() {
                    bail!(
                        "Architecture in manifest ({a}) is not compatible with built target ({:?})",
                        architecture
                    );
                }
            } else {
                debug!(
                    "Architecture not set in manifest, using {:?}",
                    &architecture
                );
                setup.insert(
                    "architecture".to_string(),
                    Value::String(architecture.to_string()),
                );
            }
        }
        Ok(manifest)
    }

    pub(crate) fn as_value(&self) -> &Value {
        &self.0
    }

    pub(crate) fn as_object(&self) -> json_ext::Result<&Map<String, Value>> {
        self.0.try_to_object()
    }

    pub(crate) fn as_object_mut(&mut self) -> json_ext::Result<&mut Map<String, Value>> {
        self.0.try_to_object_mut()
    }

    // TODO: Consider generalizing this to something like `try_get_as_str(&self, path: &[&str])`
    pub(crate) fn try_find_app_name(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("appName")
    }

    pub(crate) fn try_find_architecture(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("architecture")
    }

    pub(crate) fn try_find_friendly_name(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("friendlyName")
    }

    pub(crate) fn try_find_http_config(&self) -> json_ext::Result<&Vec<Value>> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("configuration")?
            .try_get_array("httpConfig")
    }

    pub(crate) fn try_find_param_config(&self) -> json_ext::Result<&Vec<Value>> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("configuration")?
            .try_get_array("paramConfig")
    }

    pub(crate) fn try_find_post_install_script(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("installation")?
            .try_get_str("postInstallScript")
    }

    pub(crate) fn try_find_pre_uninstall_script(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("uninstallation")?
            .try_get_str("preUninstallScript")
    }

    pub(crate) fn try_find_version(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("version")
    }

    pub(crate) fn try_find_setup_mut(&mut self) -> json_ext::Result<&mut Map<String, Value>> {
        self.as_object_mut()?
            .try_get_object_mut("acapPackageConf")?
            .try_get_object_mut("setup")
    }

    pub(crate) fn try_to_string(&self) -> anyhow::Result<String> {
        // This file is included in the EAP, so for as long as we want bit-exact output, we must
        // take care to serialize the manifest the same way as the python implementation.
        // let mut writer = BufWriter::new(String::new());
        let mut data = Vec::new();
        let mut serializer =
            Serializer::with_formatter(&mut data, PrettyFormatter::with_indent(b"    "));
        self.0.serialize(&mut serializer)?;
        Ok(ensure_ascii(&String::from_utf8(data)?))
    }
}

/// Escape non-ASCII characters the way Python's `json.dump` does with its default
/// `ensure_ascii=True`: as `\uXXXX` sequences of UTF-16 code units, lowercase hex.
///
/// In serialized JSON non-ASCII characters can only occur inside string literals, so escaping
/// them anywhere in the document is equivalent to escaping them during serialization.
fn ensure_ascii(json: &str) -> String {
    let mut escaped = String::with_capacity(json.len());
    let mut buf = [0u16; 2];
    for c in json.chars() {
        if c.is_ascii() {
            escaped.push(c);
        } else {
            for unit in c.encode_utf16(&mut buf) {
                escaped.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn ensure_ascii_escapes_like_python_json_dump() {
        // Matches `json.dumps("Åπ🦀")`.
        assert_eq!(
            ensure_ascii("\"\u{c5}\u{3c0}\u{1f980}\""),
            "\"\\u00c5\\u03c0\\ud83e\\udd80\""
        );
        assert_eq!(ensure_ascii("\"ascii\""), "\"ascii\"");
    }

    #[test]
    fn architecture_is_added_from_schema_version_1_3() {
        for schema_version in ["1.3", "1.3.0", "1.4"] {
            let manifest = Manifest::new(
                json!({
                    "schemaVersion": schema_version,
                    "acapPackageConf": {
                        "setup": {
                            "appName": "a",
                            "runMode": "never",
                            "version": "0.0.0"
                        }
                    }
                }),
                Architecture::Aarch64,
            )
            .unwrap();
            assert_eq!(
                manifest.try_find_architecture().ok(),
                Some("aarch64"),
                "schemaVersion {schema_version:?}"
            );
        }
    }

    #[test]
    fn malformed_schema_version_is_an_error_not_a_panic() {
        // A version with more segments than semver allows must not underflow the
        // padding loop; it should surface as an ordinary parse error.
        let err = Manifest::new(
            json!({
                "schemaVersion": "1.2.3.4",
                "acapPackageConf": {
                    "setup": {
                        "appName": "a",
                        "runMode": "never",
                        "version": "0.0.0"
                    }
                }
            }),
            Architecture::Aarch64,
        );
        assert!(err.is_err());
    }

    #[test]
    fn architecture_is_not_added_before_schema_version_1_3() {
        let manifest = Manifest::new(
            json!({
                "schemaVersion": "1.2",
                "acapPackageConf": {
                    "setup": {
                        "appName": "a",
                        "runMode": "never",
                        "version": "0.0.0"
                    }
                }
            }),
            Architecture::Aarch64,
        )
        .unwrap();
        manifest.try_find_architecture().unwrap_err();
    }

    #[test]
    fn try_to_string_escapes_non_ascii() {
        let manifest = Manifest::new(
            json!({
                "schemaVersion": "1.3",
                "acapPackageConf": {
                    "setup": {
                        "appName": "a",
                        "friendlyName": "\u{c5}pp \u{3a9}",
                        "runMode": "never",
                        "version": "0.0.0"
                    }
                }
            }),
            Architecture::Aarch64,
        )
        .unwrap();
        assert_eq!(
            manifest.try_to_string().unwrap(),
            r#"{
    "schemaVersion": "1.3",
    "acapPackageConf": {
        "setup": {
            "appName": "a",
            "friendlyName": "\u00c5pp \u03a9",
            "runMode": "never",
            "version": "0.0.0",
            "architecture": "aarch64"
        }
    }
}"#
        );
    }
}

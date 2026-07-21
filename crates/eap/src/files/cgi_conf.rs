//! Code for populating the `cgi.conf` file
use std::fmt::{Display, Formatter};

use log::debug;

use crate::{
    json_ext::{self, MapExt, ValueExt},
    original_manifest::OriginalManifest,
};

#[derive(Debug)]
enum Entry {
    Fast { access: String, name: String },
    Other { access: String, name: String },
}

#[derive(Debug)]
pub(crate) struct CgiConf(Vec<Entry>);

impl CgiConf {
    pub(crate) fn new(manifest: &OriginalManifest) -> anyhow::Result<Option<Self>> {
        let conf = match manifest.try_find_http_config() {
            Ok(v) => v,
            Err(json_ext::Error::KeyNotFound(_)) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // note that a non-empty httpConfig with only directory entries still
        // produces a file, albeit an empty one.
        if conf.is_empty() {
            debug!("Skipping cgi.conf, httpConfig is empty");
            return Ok(None);
        }

        let mut entries = Vec::new();
        for obj in conf.iter() {
            let obj = obj.try_to_object()?;

            let kind = obj.try_get_str("type")?;
            if kind == "directory" {
                debug!("Skipping httpConfig of type directory");
                continue;
            }

            let name = obj.try_get_str("name")?.trim_start_matches('/').to_string();

            let access = match obj.try_get_str("access")? {
                "admin" => "administrator",
                access => access,
            }
            .to_string();

            entries.push(match kind {
                "fastCgi" => Entry::Fast { access, name },
                _ => Entry::Other { access, name },
            })
        }
        Ok(Some(Self(entries)))
    }
}

impl Display for CgiConf {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for cgi in &self.0 {
            match &cgi {
                Entry::Fast { access, name } => writeln!(f, "{access} /{name} fastCgi")?,
                Entry::Other { access, name } => writeln!(f, "{access} /{name}")?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    fn manifest_with_http_config(http_config: Value) -> OriginalManifest {
        OriginalManifest::new(json!({
            "schemaVersion": "1.3",
            "acapPackageConf": {
                "setup": {
                    "appName": "a",
                    "runMode": "never",
                    "version": "0.0.0"
                },
                "configuration": {
                    "httpConfig": http_config
                }
            }
        }))
    }

    #[test]
    fn cgi_conf_is_omitted_when_http_config_is_empty() {
        let manifest = manifest_with_http_config(json!([]));
        assert!(CgiConf::new(&manifest).unwrap().is_none());
    }

    #[test]
    fn cgi_conf_is_empty_but_present_when_http_config_has_only_directory_entries() {
        // Unlike an empty httpConfig, one containing only directory entries still produces a
        // file, albeit an empty one, in both implementations.
        let manifest = manifest_with_http_config(json!([
            {"type": "directory", "name": "html", "access": "viewer"}
        ]));
        let conf = CgiConf::new(&manifest).unwrap().unwrap();
        assert_eq!(conf.to_string(), "");
    }
}

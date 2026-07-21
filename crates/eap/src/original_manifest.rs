//! The application manifest as authored.
use serde_json::{Map, Value};

use crate::json_ext::{self, MapExt, ValueExt};

/// The application manifest as authored: the input that every packaged file is generated from.
#[derive(Debug)]
pub(crate) struct OriginalManifest(Value);

impl OriginalManifest {
    pub(crate) fn new(manifest: Value) -> Self {
        Self(manifest)
    }

    pub(crate) fn as_value(&self) -> &Value {
        &self.0
    }

    pub(crate) fn as_object(&self) -> json_ext::Result<&Map<String, Value>> {
        self.0.try_to_object()
    }

    pub(crate) fn try_find_app_name(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("appName")
    }

    pub(crate) fn try_find_friendly_name(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("friendlyName")
    }

    pub(crate) fn try_find_version(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("version")
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
}

//! A model of the source code of an application.
//!
//! The model is not the source code itself but a description of it, structured so that every
//! source tree it can describe is one that `acap-build` implementations should be able to
//! package: strategies explore the space of interesting applications instead of wasting cases
//! on trees that are rejected before any packaging logic runs.
//!
//! The model currently captures
//! - the manifest, including where on disk it is located;
//! - the mandatory files: the license and the executable named by the manifest;
//! - the optional `html` directory, which implementations pick up without being told;
//! - additional files, which are packaged only when named by `--additional-file`.

use std::{collections::BTreeSet, fs, os::unix::fs::PermissionsExt, path::Path};

use proptest::{
    arbitrary::{any, Arbitrary},
    prelude::{prop, BoxedStrategy, Just, Strategy},
    prop_oneof,
};
use serde_json::{json, Map, Value};

/// The name that implementations use when no manifest is named explicitly.
pub const DEFAULT_MANIFEST_NAME: &str = "manifest.json";

/// Directory names that implementations treat specially and that would shadow other files if
/// used as file names.
const RESERVED_NAMES: [&str; 3] = ["lib", "html", "declarations"];

fn write_file(dir: &Path, rel_path: &str, content: &[u8], executable: bool) -> anyhow::Result<()> {
    let path = dir.join(rel_path);
    fs::write(&path, content)?;
    let mode = if executable { 0o755 } else { 0o644 };
    fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

fn file_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,8}".prop_filter(
        "file name must not shadow a reserved directory name",
        |name| !RESERVED_NAMES.contains(&name.as_str()),
    )
}

/// The part of the manifest content that the model can vary.
#[derive(Clone, Debug)]
pub struct Manifest {
    pub schema_version: &'static str,
    pub app_name: String,
    pub version: String,
    pub friendly_name: Option<String>,
}

impl Manifest {
    fn json(&self) -> Value {
        let mut setup = Map::new();
        setup.insert("appName".into(), json!(self.app_name));
        if let Some(v) = &self.friendly_name {
            setup.insert("friendlyName".into(), json!(v));
        }
        setup.insert("runMode".into(), json!("never"));
        setup.insert("version".into(), json!(self.version));

        json!({
            "schemaVersion": self.schema_version,
            "acapPackageConf": { "setup": Value::Object(setup) },
        })
    }
}

impl Arbitrary for Manifest {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with((): ()) -> Self::Strategy {
        (
            // 1.3 and 1.3.0 sit on the boundary at which the architecture field starts being added;
            // 1.7.0 is a control that both implementations handle identically.
            // TODO: Consider sampling from all known versions and maybe some novel strings
            prop_oneof![Just("1.3"), Just("1.3.0"), Just("1.7.0")],
            file_name(),
            (0u8..20, 0u8..20, 0u8..100).prop_map(|(a, b, c)| format!("{a}.{b}.{c}")),
            prop::option::of(prop_oneof![
                "[A-Za-z][A-Za-z0-9 ]{0,10}",
                Just("\u{c5}pp \u{3a9}".to_string()),
            ]),
        )
            .prop_map(
                |(schema_version, app_name, version, friendly_name)| Manifest {
                    schema_version,
                    app_name,
                    version,
                    friendly_name,
                },
            )
            .boxed()
    }
}

#[derive(Clone, Debug)]
pub struct Source {
    pub manifest: Manifest,
    /// The name of the file that the manifest is written to.
    ///
    /// When this is not [`DEFAULT_MANIFEST_NAME`] the invocation must name it with `--manifest`.
    pub manifest_name: String,
    /// Plain files that are packaged only when named by `--additional-file`.
    pub additional_files: BTreeSet<String>,
    /// Whether an `html` directory exists; implementations pick it up without being told.
    pub html: bool,
}

impl Source {
    pub fn materialize_in(&self, dir: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(dir)?;
        fs::write(
            dir.join(&self.manifest_name),
            serde_json::to_string_pretty(&self.manifest.json())?,
        )?;
        fs::write(dir.join("LICENSE"), "All rights reserved.\n")?;
        write_file(dir, &self.manifest.app_name, b"#!/bin/sh\nexit 0\n", true)?;
        for name in &self.additional_files {
            write_file(dir, name, b"An additional file.\n", false)?;
        }
        if self.html {
            fs::create_dir(dir.join("html"))?;
            fs::write(dir.join("html").join("index.html"), "<!DOCTYPE html>\n")?;
        }
        Ok(())
    }
}

impl Arbitrary for Source {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with((): ()) -> Self::Strategy {
        (
            any::<Manifest>(),
            prop_oneof![
                3 => Just(DEFAULT_MANIFEST_NAME.to_string()),
                1 => Just("unconventional-manifest.json".to_string()),
            ],
            prop::collection::btree_set(file_name(), 0..3),
            any::<bool>(),
        )
            .prop_map(|(manifest, manifest_name, mut additional_files, html)| {
                // The executable is written last so a collision would corrupt the app; the
                // space of colliding names is not interesting enough to explore.
                additional_files.remove(&manifest.app_name);
                Source {
                    manifest,
                    manifest_name,
                    additional_files,
                    html,
                }
            })
            .boxed()
    }
}

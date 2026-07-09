#![forbid(unsafe_code)]
//! Library for creating Embedded Application Packages (EAPs).
use std::{
    collections::HashSet,
    ffi::OsString,
    fs,
    io::Write,
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{bail, ensure, Context};
use log::debug;
use semver::Version;
use serde_json::Value;

use crate::files::{
    cgi_conf::CgiConf, manifest::Manifest, package_conf::PackageConf, param_conf::ParamConf,
};

mod archive;
mod command_utils;
mod json_ext;
mod schema;

mod files;

pub use schema::SchemaSource;

use crate::archive::EquivalentArchiveBuilder;

/// The location where the ACAP SDK is installed by default.
///
/// Used as the default location for resolving manifest schemas; see [`SchemaSource::Resolve`].
pub const DEFAULT_ACAP_SDK_LOCATION: &str = "/opt/axis/";

/// A modification time, in seconds after the Unix epoch, that the tar headers in the EAP can
/// represent.
///
/// The 12-byte numeric header fields hold 11 octal digits; GNU tar silently encodes larger
/// values with its base-256 extension, which not every unpacker understands, so conversion
/// fails for values above [`Self::MAX`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Mtime(u64);

impl Mtime {
    /// The largest value that the tar headers can portably represent.
    pub const MAX: Self = Self((1 << 33) - 1);
}

impl TryFrom<u64> for Mtime {
    type Error = anyhow::Error;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        // A larger value is almost always a millisecond timestamp; catching it here gives a
        // targeted error where GNU tar would silently encode it in base-256.
        ensure!(
            value <= Self::MAX.0,
            "{value} does not fit in the tar headers' 11 octal digits (max {}); if it is in \
             milliseconds, convert it to seconds",
            Self::MAX.0
        );
        Ok(Self(value))
    }
}

// TODO: Find a better way to support reproducible builds
fn copy<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    dst: Q,
    copy_permissions: bool,
) -> anyhow::Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    if dst.symlink_metadata().is_ok() {
        bail!("Path already exists {dst:?}");
    }
    if src.is_symlink() {
        // Recreate the symlink rather than following it. A symlink's own mode is not portably
        // settable, so `copy_permissions` does not apply here.
        let target = fs::read_link(src)?;
        symlink(target, dst)?;
    } else if copy_permissions {
        fs::copy(src, dst)?;
    } else {
        let mut src = fs::File::open(src)?;
        let mut dst = fs::File::create(dst)?;
        std::io::copy(&mut src, &mut dst)?;
    }
    Ok(())
}

fn copy_recursively(src: &Path, dst: &Path, copy_permissions: bool) -> anyhow::Result<()> {
    if !src.is_dir() {
        copy(src, dst, copy_permissions)?;
        debug!("Created reg {dst:?}");
        return Ok(());
    }
    match fs::create_dir(dst) {
        Ok(()) => {
            debug!("Created dir {dst:?}");
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        copy_recursively(
            &entry.path(),
            &dst.join(entry.file_name()),
            copy_permissions,
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcapBuildImpl {
    /// Use the native, equivalent implementation.
    Equivalent,
}

impl FromStr for AcapBuildImpl {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "equivalent" => Ok(Self::Equivalent),
            _ => bail!("Expected 'equivalent', but found {s:?}"),
        }
    }
}

pub struct AppBuilder<'a> {
    preserve_permissions: bool,
    staging_dir: &'a Path,
    manifest: Manifest,
    files: Vec<String>,
    default_architecture: Architecture,
    app_name: String,
    acap_build_impl: AcapBuildImpl,
    schema: SchemaSource,
    mtime: Mtime,
}

impl<'a> AppBuilder<'a> {
    pub fn new(
        preserve_permissions: bool,
        staging_dir: &'a Path,
        manifest: &Path,
        default_architecture: Architecture,
    ) -> anyhow::Result<Self> {
        let manifest: Value = serde_json::from_reader(fs::File::open(manifest)?)?;
        let manifest = Manifest::new(manifest, default_architecture)?;
        let app_name = manifest.try_find_app_name()?.to_string();
        Ok(Self {
            preserve_permissions,
            staging_dir,
            manifest,
            app_name,
            files: Vec::new(),
            default_architecture,
            acap_build_impl: AcapBuildImpl::Equivalent,
            schema: Default::default(),
            mtime: Mtime::default(),
        })
    }

    /// Select the implementation used to build the EAP.
    ///
    /// Defaults to [`AcapBuildImpl::Equivalent`].
    pub fn implementation(&mut self, acap_build_impl: AcapBuildImpl) -> &mut Self {
        self.acap_build_impl = acap_build_impl;
        self
    }

    /// Select how to validate the application manifest before building the EAP.
    ///
    /// Defaults to [`SchemaSource::None`]
    pub fn schema(&mut self, schema: SchemaSource) -> &mut Self {
        self.schema = schema;
        self
    }

    /// Set the modification time stamped on every archive member.
    ///
    /// Defaults to the Unix epoch. Reading this from the environment (e.g. from
    /// `SOURCE_DATE_EPOCH`) or the clock is left to the caller so that the library stays
    /// deterministic given its inputs.
    pub fn mtime(&mut self, mtime: Mtime) -> &mut Self {
        self.mtime = mtime;
        self
    }

    /// Add a file to the EAP.
    pub fn add(&mut self, path: &Path) -> anyhow::Result<&mut Self> {
        let name = path
            .file_name()
            .context("file has no name")?
            .to_str()
            .context("file name is not a string")?;
        self.add_as(path, name)?;
        Ok(self)
    }

    /// Add all files in a directory to the EAP.
    pub fn add_from(&mut self, dir: &Path) -> anyhow::Result<&mut Self> {
        let mut entries = fs::read_dir(dir)?
            .map(|res| res.map(|e| e.path()))
            .collect::<std::io::Result<Vec<PathBuf>>>()?;
        entries.sort();
        for entry in entries {
            let name = entry
                .file_name()
                .context("file has no name")?
                .to_str()
                .context("file name is not a string")?;
            self.add_as(&entry, name)?;
        }
        Ok(self)
    }

    // TODO: Remove the file system copy
    pub fn add_as(&mut self, path: &Path, name: &str) -> anyhow::Result<PathBuf> {
        let dst = self.staging_dir.join(name);
        if dst.symlink_metadata().is_ok() {
            bail!("Cannot add {path:?} because {name} already exists");
        }
        copy_recursively(path, &dst, self.preserve_permissions)?;
        self.files.push(name.to_string());
        if name == self.app_name && !self.preserve_permissions {
            let mut permissions = fs::metadata(&dst)?.permissions();
            let mode = permissions.mode();
            permissions.set_mode(mode | 0o111);
            fs::set_permissions(&dst, permissions)?;
        }
        debug!("Added {name} from {path:?}");
        Ok(dst)
    }

    /// Add the **mandatory** executable to the EAP.
    pub fn add_exe(&mut self, reg: &Path) -> anyhow::Result<&mut Self> {
        // TODO: Consider refactoring or changing to avoid cloning.
        let app_name = self.app_name.clone();
        self.add_as(reg, &app_name)?;
        Ok(self)
    }

    /// Build the EAP and return its path.
    pub fn build(self) -> anyhow::Result<OsString> {
        match self.acap_build_impl {
            AcapBuildImpl::Equivalent => {
                debug!("Bypassing acap-build");
                self.build_native()
            }
        }
    }

    fn build_native(self) -> anyhow::Result<OsString> {
        schema::validate(self.manifest.as_value(), &self.schema)
            .context("validating manifest against schema")?;

        let Self {
            staging_dir,
            manifest,

            default_architecture,
            app_name,
            ..
        } = &self;

        // Compute file name
        let package_name = match manifest.try_find_friendly_name() {
            Ok(v) => v,
            Err(json_ext::Error::KeyNotFound(_)) => app_name.as_str(),
            Err(e) => return Err(e.into()),
        }
        .replace(' ', "_");
        let Version {
            major,
            minor,
            patch,
            ..
        } = manifest.try_find_version().context("no version")?.parse()?;

        let arch = match manifest.try_find_architecture() {
            Ok(v) => v,
            Err(json_ext::Error::KeyNotFound(_)) => default_architecture.nickname(),
            Err(e) => return Err(e.into()),
        };
        let eap_file_name = format!("{package_name}_{major}_{minor}_{patch}_{arch}.eap");

        // Generate derived files
        let package_conf =
            PackageConf::new(manifest, &self.other_files(), *default_architecture)?.to_string();
        fs::File::create_new(staging_dir.join("package.conf"))?
            .write_all(package_conf.as_bytes())?;

        let param_conf = match ParamConf::new(manifest)? {
            None => {
                // If there is no param.conf, `eap-create.sh` creates one
                debug!("Creating empty param.conf");
                String::new()
            }
            Some(v) => v.to_string(),
        };
        fs::File::create_new(staging_dir.join("param.conf"))?.write_all(param_conf.as_bytes())?;

        match CgiConf::new(manifest)? {
            None => {
                debug!("Skipping cgi.conf")
            }
            Some(cgi_conf) => {
                fs::File::create_new(staging_dir.join("cgi.conf"))?
                    .write_all(cgi_conf.to_string().as_bytes())?;
            }
        }

        // This file is included in the EAP, so for as long as we want bit-exact output, we must
        // take care to serialize the manifest the same way as the python implementation.
        let manifest_file = staging_dir.join("manifest.json");
        fs::File::create_new(&manifest_file)?.write_all(manifest.try_to_string()?.as_bytes())?;
        // Replicate the permissions that temporary files get by default.
        let mut permissions = fs::metadata(&manifest_file)?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&manifest_file, permissions)?;

        // Create the archive
        let mut tar = EquivalentArchiveBuilder::new(staging_dir, &eap_file_name, self.mtime);

        for name in self.section_1_files() {
            if staging_dir.join(name).symlink_metadata().is_ok() {
                tar.file(name);
            }
        }

        tar.files(self.other_files().as_slice());

        // TODO: Consider implementing support for `httpd.conf.local.*` and `mime.types.local.*`.

        for name in self.section_4_files() {
            if staging_dir.join(name).symlink_metadata().is_ok() {
                tar.file(name);
            }
        }

        tar.run_with_logged_stdout()?;

        Ok(OsString::from(eap_file_name))
    }

    // These sections are probably relevant only for the equivalent implementation;
    // Once unpacked on device the order of files or the reason they were included is not important
    // (even though some files are nonetheless treated specially).
    // The sections don't have any semantics, they are just partitions that can be composed to
    // create meaningful or useful lists of names.

    fn section_1_files(&self) -> Vec<&str> {
        [
            Some(self.app_name.as_str()),
            Some("package.conf"),
            Some("param.conf"),
            Some("LICENSE"),
            Some("manifest.json"),
            self.manifest.try_find_post_install_script().ok(),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn section_2_files(&self) -> Vec<&str> {
        let known_files: HashSet<_> = [
            self.section_1_files(),
            self.section_3_files(),
            self.section_4_files(),
        ]
        .into_iter()
        .flatten()
        .collect();

        self.files
            .iter()
            .map(String::as_str)
            .filter(|f| !known_files.contains(f))
            .collect()
    }

    fn section_3_files(&self) -> Vec<&str> {
        [self.manifest.try_find_pre_uninstall_script().ok()]
            .into_iter()
            .flatten()
            .collect()
    }

    fn section_4_files(&self) -> Vec<&str> {
        vec!["html", "declarations", "lib", "cgi.conf"]
    }

    /// Other files for the `package.conf` file.
    fn other_files(&self) -> Vec<&str> {
        [self.section_2_files(), self.section_3_files()].concat()
    }

    /// Return the name of files that must be added using [`Self::add`].
    pub fn mandatory_files(&self) -> Vec<String> {
        [
            Some(self.app_name.as_str()),
            Some("LICENSE"),
            self.manifest.try_find_post_install_script().ok(),
            self.manifest.try_find_pre_uninstall_script().ok(),
        ]
        .into_iter()
        .flatten()
        .map(str::to_string)
        .collect()
    }

    /// Return the name of files that should be added using [`Self::add`].
    pub fn optional_files(&self) -> Vec<String> {
        ["html", "declarations", "lib"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Architecture {
    Aarch64,
    Armv7hf,
}

impl Architecture {
    pub fn triple(&self) -> &'static str {
        match self {
            Architecture::Aarch64 => "aarch64-unknown-linux-gnu",
            Architecture::Armv7hf => "thumbv7neon-unknown-linux-gnueabihf",
        }
    }

    pub fn nickname(&self) -> &'static str {
        match self {
            Self::Aarch64 => "aarch64",
            Self::Armv7hf => "armv7hf",
        }
    }
}

impl FromStr for Architecture {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "aarch64" => Ok(Self::Aarch64),
            "arm" => Ok(Self::Armv7hf),
            _ => Err(anyhow::anyhow!("Unrecognized variant {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mtime_can_be_constructed_only_from_values_that_fit_in_the_headers() {
        let Mtime(_) = Mtime::try_from(Mtime::MAX.0).unwrap();
        assert!(Mtime::try_from(Mtime::MAX.0 + 1).is_err());
    }

    #[test]
    fn copy_recreates_symlinks() {
        for copy_permissions in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            fs::write(dir.path().join("target.txt"), "hello").unwrap();
            let src = dir.path().join("link.txt");
            symlink("target.txt", &src).unwrap();

            let dst = dir.path().join("copy.txt");
            copy(&src, &dst, copy_permissions).unwrap();

            assert!(dst.symlink_metadata().unwrap().file_type().is_symlink());
            assert_eq!(fs::read_link(&dst).unwrap(), Path::new("target.txt"));
        }
    }
}

use std::{
    fmt::{Display, Formatter},
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use clap::{Parser, ValueEnum};
use log::debug;
use rs4a_eap::{AcapBuildImpl, AppBuilder, Architecture, Mtime, SchemaSource};
use tempdir::TempDir;

mod conservative;

pub use conservative::ConservativeRejection;

/// The location where the ACAP SDK is installed by default.
pub const DEFAULT_ACAP_SDK_LOCATION: &str = "/opt/axis/";

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum OpenEmbeddedTargetArchitecture {
    Aarch64,
    Arm,
}

impl From<OpenEmbeddedTargetArchitecture> for Architecture {
    fn from(value: OpenEmbeddedTargetArchitecture) -> Self {
        match value {
            OpenEmbeddedTargetArchitecture::Aarch64 => Self::Aarch64,
            OpenEmbeddedTargetArchitecture::Arm => Self::Armv7hf,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, ValueEnum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
#[clap(rename_all = "kebab-case")]
pub enum BuildOption {
    #[default]
    Make,
    NoBuild,
}

impl Display for BuildOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Make => write!(f, "make"),
            Self::NoBuild => write!(f, "no-build"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Cli {
    pub path: PathBuf,
    /// Build tool, if any, to run before packaging.
    #[clap(default_value_t, long, short)]
    pub build: BuildOption,
    /// Location of the manifest relative to the path argument.
    #[clap(long, short, default_value = "manifest.json")]
    pub manifest: PathBuf,
    /// Additional files to include in the package.
    /// May be specified multiple times
    ///
    /// If it matches the archiver's exclude patterns (`*~` and VCS names like `.git`),
    /// then it's silently omitted from the archive but still listed in `package.conf`.
    // TODO: Consider pushing this quirk out of the library and into this binary
    #[clap(long, short)]
    pub additional_file: Vec<PathBuf>,
    /// Disable validation of manifest file against manifest schema.
    #[clap(long)]
    pub disable_manifest_validation: bool,
    #[clap(long, env = "OECORE_TARGET_ARCH")]
    pub oecore_target_arch: OpenEmbeddedTargetArchitecture,
    /// Used by `--conservative`.
    #[clap(long, env = "OECORE_NATIVE_SYSROOT")]
    pub oecore_native_sysroot: Option<PathBuf>,
    /// Used by `--conservative`.
    #[clap(long, env = "SDKTARGETSYSROOT")]
    pub sdk_target_sysroot: Option<PathBuf>,
    #[clap(
        long,
        env = "ACAP_SDK_LOCATION",
        default_value = DEFAULT_ACAP_SDK_LOCATION
    )]
    pub acap_sdk_location: PathBuf,
    /// Time to stamp on every archive member, in seconds after the Unix epoch.
    ///
    /// Defaults to the current time.
    #[clap(long, env = "SOURCE_DATE_EPOCH", value_parser = parse_mtime)]
    pub source_date_epoch: Option<Mtime>,
    /// Implementation used to package the EAP.
    #[clap(long = "impl", env = "ACAP_BUILD_IMPL", default_value_t = AcapBuildImpl::Equivalent)]
    pub acap_build_impl: AcapBuildImpl,
    /// Reject inputs for which the correct behavior is ambiguous.
    #[clap(long)]
    pub conservative: bool,
}

fn parse_mtime(s: &str) -> anyhow::Result<Mtime> {
    s.trim().parse::<u64>()?.try_into()
}

/// The directory under an installed ACAP SDK that holds the manifest schemas.
///
/// The schemas are nested deeply within the reference SDK layout; deriving the path here keeps that
/// knowledge in the binary rather than in the `eap` library, whose `SchemaSource::Resolve` takes the
/// schemas dir directly.
fn sdk_schema_dir(acap_sdk_location: &Path) -> PathBuf {
    acap_sdk_location
        .join("acapsdk")
        .join("axis-acap-manifest-tools")
        .join("schema")
        .join("schemas")
}

impl Cli {
    pub fn exec(self) -> anyhow::Result<String> {
        if self.conservative {
            conservative::error_for_rejection(&self)?;
        }

        let Self {
            path,
            build,
            manifest,
            additional_file,
            disable_manifest_validation,
            oecore_target_arch,
            oecore_native_sysroot: _,
            sdk_target_sysroot: _,
            acap_sdk_location,
            source_date_epoch,
            acap_build_impl,
            conservative: _,
        } = self;

        let schema = if disable_manifest_validation {
            SchemaSource::None
        } else {
            SchemaSource::Resolve(sdk_schema_dir(&acap_sdk_location))
        };

        // Reading the clock here, and the environment via clap, keeps the library deterministic
        // given its inputs. Falling back to the current time matches the upstream tool.
        let mtime = match source_date_epoch {
            Some(value) => value,
            None => Mtime::try_from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .context("reading current time")?
                    .as_secs(),
            )
            .context("converting current time")?,
        };
        match build {
            BuildOption::Make => assert!(Command::new("make")
                .status()
                .context("subprocess make failed")?
                .success()),
            BuildOption::NoBuild => {
                debug!("no build");
            }
        }

        let manifest = path.join(&manifest);

        let staging_dir = TempDir::new_in(&path, "acap-build")?;
        let mut builder = AppBuilder::new(
            true,
            staging_dir.path(),
            &manifest,
            oecore_target_arch.into(),
        )?;

        builder.schema(schema);
        builder.mtime(mtime);
        builder.implementation(acap_build_impl);

        for name in builder.mandatory_files() {
            builder.add(&path.join(name))?;
        }

        for name in builder.optional_files() {
            let file = path.join(name);
            if file.symlink_metadata().is_ok() {
                builder.add(&file)?;
            }
        }

        for additional_file in additional_file {
            builder.add(&path.join(additional_file))?;
        }

        let eap_file_name = builder.build()?;
        let eap_file_path = path.join(&eap_file_name);
        fs::copy(staging_dir.path().join(&eap_file_name), &eap_file_path)?;

        Ok(eap_file_path.display().to_string())
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn cmd() -> clap::Command {
        Cli::command()
    }

    #[test]
    fn cli_is_valid() {
        cmd().debug_assert();
    }
}

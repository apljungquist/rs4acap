use std::{
    fmt::{Display, Formatter},
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use clap::{Parser, ValueEnum};
use log::debug;
use rs4a_eap::{AcapBuildImpl, AppBuilder, Architecture, Mtime, SchemaSource};
use tempdir::TempDir;

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
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

#[derive(Clone, Debug, Parser)]
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
    #[clap(
        long,
        env = "ACAP_SDK_LOCATION",
        default_value = rs4a_eap::DEFAULT_ACAP_SDK_LOCATION
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
}

fn parse_mtime(s: &str) -> anyhow::Result<Mtime> {
    s.trim().parse::<u64>()?.try_into()
}

impl Cli {
    pub fn exec(self) -> anyhow::Result<String> {
        let Self {
            path,
            build,
            manifest,
            additional_file,
            disable_manifest_validation,
            oecore_target_arch,
            acap_sdk_location,
            source_date_epoch,
            acap_build_impl,
        } = self;

        let schema = if disable_manifest_validation {
            SchemaSource::None
        } else {
            SchemaSource::Resolve(acap_sdk_location)
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

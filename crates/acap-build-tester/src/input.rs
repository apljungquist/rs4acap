//! The input that affects the execution of an `acap-build` implementation.

use std::{
    fs,
    path::{Path, PathBuf},
};

use acap_build::{Architecture, BuildOption, Cli};
use anyhow::{anyhow, Context};
use clap::ValueEnum;
use proptest::{
    arbitrary::any,
    prelude::{BoxedStrategy, Just, Strategy},
    prop_oneof,
};
use rs4a_eap::{AcapBuildImpl, Mtime, DEFAULT_ACAP_SDK_LOCATION};
use serde::{Deserialize, Serialize};

use crate::source::{Source, DEFAULT_MANIFEST_NAME};

/// Name of the file, alongside an example's source, that records how to invoke `acap-build`.
pub const INVOCATION_FILE: &str = "invocation.json";

/// The complete, known input to an `acap-build` implementation.
#[derive(Clone, Debug)]
pub struct Input {
    pub source: Source,
    pub invocation: Cli,
}

pub fn arbitrary_input(oecore_target_arch: Architecture) -> BoxedStrategy<Input> {
    // Taken from the environment rather than generated: the reference reads it from the SDK
    // environment, so the candidate must see the same value to agree with it.
    let axis_os_version = std::env::var("AXIS_OS_VERSION").ok();
    (
        any::<Source>(),
        any::<bool>(),
        // Nonzero to catch implementations that ignore the variable and use the (zero-ish)
        // default of their tar library; small enough to fit every timestamp encoding.
        prop_oneof![Just(0u64), Just(1234567890)],
    )
        .prop_map(move |(source, disable_manifest_validation, epoch)| Input {
            invocation: Cli {
                // A placeholder; each implementation builds in a scratch directory of its own.
                // Easy to forget to overwriting, so avoiding this would be one advantage of moving
                // away from the `Cli` as input model
                path: PathBuf::new(),
                build: BuildOption::NoBuild,
                manifest: PathBuf::from(&source.manifest_name),
                additional_file: source.additional_files.iter().map(PathBuf::from).collect(),
                disable_manifest_validation,
                // Taken from the environment rather than generated for now to efficiently generate
                // interesting inputs given a realistic environment.
                // TODO: Consider varying this, including leaving it unset.
                oecore_target_arch,
                axis_os_version: axis_os_version.clone(),
                // Only the default is generated: the reference does not read it, so any other
                // location would make only the candidate use different schema which is an
                // unnecessary potential source of divergence.
                acap_sdk_location: PathBuf::from(DEFAULT_ACAP_SDK_LOCATION),
                // Always set: `None` falls back to the current time, which the two
                // implementations would sample at different moments.
                source_date_epoch: Some(
                    Mtime::try_from(epoch).expect("generated values fit in the tar headers"),
                ),
                acap_build_impl: AcapBuildImpl::Equivalent,
            },
            source,
        })
        .boxed()
}

/// A recorded `acap-build` invocation, stored alongside an example so replay can reproduce it.
///
/// Only the arguments are recorded. The target architecture and AXIS OS version are deliberately
/// left out: the reference derives them from the SDK environment it runs in (the package
/// architecture, for instance, follows the environment even when `OECORE_TARGET_ARCH` is
/// overridden), so an example must be built for whichever environment replay runs in rather than
/// one pinned on disk. Replay therefore takes them from the ambient environment, the same for
/// both implementations.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredInvocation {
    build: String,
    manifest: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    additional_file: Vec<String>,
    #[serde(default)]
    disable_manifest_validation: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_date_epoch: Option<u64>,
}

impl StoredInvocation {
    fn from_cli(cli: &Cli) -> Self {
        Self {
            build: cli.build.to_string(),
            manifest: cli.manifest.display().to_string(),
            additional_file: cli
                .additional_file
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
            disable_manifest_validation: cli.disable_manifest_validation,
            source_date_epoch: cli.source_date_epoch.map(u64::from),
        }
    }

    fn into_cli(
        self,
        path: PathBuf,
        oecore_target_arch: Architecture,
        axis_os_version: Option<String>,
    ) -> anyhow::Result<Cli> {
        Ok(Cli {
            path,
            build: BuildOption::from_str(&self.build, false).map_err(|e| anyhow!(e))?,
            manifest: PathBuf::from(self.manifest),
            additional_file: self.additional_file.into_iter().map(PathBuf::from).collect(),
            disable_manifest_validation: self.disable_manifest_validation,
            oecore_target_arch,
            axis_os_version,
            acap_sdk_location: PathBuf::from(DEFAULT_ACAP_SDK_LOCATION),
            // A recorded invocation must pin the timestamp: with `None`, both implementations
            // would fall back to wall-clock time, sampled at different instants, and their archive
            // mtimes would diverge, making replay report a spurious mismatch. Reject a missing
            // value rather than replay flakily.
            source_date_epoch: Some(Mtime::try_from(self.source_date_epoch.context(
                "recorded invocation is missing `sourceDateEpoch`; replay needs it pinned so both \
                 implementations stamp the same time",
            )?)?),
            acap_build_impl: AcapBuildImpl::Equivalent,
        })
    }
}

impl Input {
    /// Write the example's source and its invocation into `dir`, replacing any existing contents.
    ///
    /// The directory is cleared first so that reusing a `--save-failing` target does not leave
    /// stale files from an earlier example (an `html/` directory, a differently named manifest or
    /// executable, extra files) that would change what replay later builds.
    pub fn save_to(&self, dir: &Path) -> anyhow::Result<()> {
        match fs::remove_dir_all(dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(anyhow::Error::from(e).context(format!("clearing {dir:?}"))),
        }
        self.source.materialize_in(dir)?;
        let json = serde_json::to_string_pretty(&StoredInvocation::from_cli(&self.invocation))?;
        fs::write(dir.join(INVOCATION_FILE), format!("{json}\n"))
            .with_context(|| format!("writing {INVOCATION_FILE} to {dir:?}"))?;
        Ok(())
    }
}

/// Load the invocation recorded for the example at `dir`, building in `path`.
///
/// The architecture and AXIS OS version come from the ambient environment (see
/// [`StoredInvocation`]). Examples created before the invocation was recorded have no
/// [`INVOCATION_FILE`]; for them the arguments fall back to the values replay always used before
/// invocations were recorded: no build step, the default manifest name, no extra files, manifest
/// validation disabled, and the timestamp pinned to the Unix epoch.
pub fn load_invocation(
    dir: &Path,
    path: PathBuf,
    oecore_target_arch: Architecture,
    axis_os_version: Option<String>,
) -> anyhow::Result<Cli> {
    let invocation_path = dir.join(INVOCATION_FILE);
    match fs::read_to_string(&invocation_path) {
        Ok(text) => serde_json::from_str::<StoredInvocation>(&text)
            .with_context(|| format!("parsing {invocation_path:?}"))?
            .into_cli(path, oecore_target_arch, axis_os_version),
        // Examples predating INVOCATION_FILE were always built with these arguments; route them
        // through `into_cli` so the environment-derived and constant fields stay defined once.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => StoredInvocation {
            build: BuildOption::NoBuild.to_string(),
            manifest: DEFAULT_MANIFEST_NAME.to_string(),
            additional_file: Vec::new(),
            disable_manifest_validation: true,
            source_date_epoch: Some(0),
        }
        .into_cli(path, oecore_target_arch, axis_os_version),
        Err(e) => Err(anyhow::Error::from(e).context(format!("reading {invocation_path:?}"))),
    }
}

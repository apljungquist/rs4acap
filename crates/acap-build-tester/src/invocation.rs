use std::{ffi::OsStr, path::PathBuf, process::Command};

use acap_build::{Cli, OpenEmbeddedTargetArchitecture};
use clap::ValueEnum;

use crate::output::Output;

#[derive(Clone, clap::Parser)]
pub struct Environment {
    /// Passed through to implementations
    #[clap(long, env = "OECORE_TARGET_ARCH")]
    pub(crate) oecore_target_arch: OpenEmbeddedTargetArchitecture,
    /// Passed through to implementations
    #[clap(long, env = "OECORE_NATIVE_SYSROOT")]
    pub(crate) oecore_native_sysroot: Option<PathBuf>,
    /// Passed through to implementations
    #[clap(long, env = "SDKTARGETSYSROOT")]
    pub(crate) sdk_target_sysroot: Option<PathBuf>,
}

/// Run an `acap-build` implementation in a sub-process.
pub fn build_with<S: AsRef<OsStr>>(program: S, cli: Cli) -> anyhow::Result<Output> {
    let Cli {
        path,
        build,
        manifest,
        additional_file,
        disable_manifest_validation,
        oecore_target_arch,
        oecore_native_sysroot,
        sdk_target_sysroot,
        source_date_epoch,
        // Not part of the reference interface
        acap_sdk_location: _,
        acap_build_impl: _,
        conservative: _,
    } = cli;

    let mut command = Command::new(program);

    // Environment variables
    command.env(
        "OECORE_TARGET_ARCH",
        oecore_target_arch
            .to_possible_value()
            .expect("no architecture variant is skipped")
            .get_name(),
    );

    match oecore_native_sysroot {
        Some(v) => command.env("OECORE_NATIVE_SYSROOT", v),
        None => command.env_remove("OECORE_NATIVE_SYSROOT"),
    };

    match sdk_target_sysroot {
        Some(v) => command.env("SDKTARGETSYSROOT", v),
        None => command.env_remove("SDKTARGETSYSROOT"),
    };

    match source_date_epoch {
        Some(epoch) => command.env("SOURCE_DATE_EPOCH", u64::from(epoch).to_string()),
        None => command.env_remove("SOURCE_DATE_EPOCH"),
    };

    // Arguments
    command.arg("--build").arg(build.to_string());
    command.arg("--manifest").arg(&manifest);
    for additional_file in &additional_file {
        command.arg("--additional-file").arg(additional_file);
    }
    if disable_manifest_validation {
        command.arg("--disable-manifest-validation");
    }

    let output = command
        // TODO: Consider testing other working dir different from manifest dir
        .arg(".")
        .current_dir(&path)
        .env_remove("RUST_LOG")
        // cargo sets LD_LIBRARY_PATH for the programs it runs,
        // which interferes with the reference implementation.
        .env_remove("LD_LIBRARY_PATH")
        .output()?;
    Output::from_command(output, &path)
}

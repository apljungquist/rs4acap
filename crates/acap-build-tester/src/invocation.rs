use std::process::Command;

use acap_build::Cli;
use clap::ValueEnum;

use crate::output::Output;

/// Run the workspace `acap-build` in-process.
///
/// GNU `tar` must be on the `PATH`.
pub fn build_with_candidate(cli: Cli) -> anyhow::Result<Output> {
    let dir = cli.path.clone();
    let result = cli.exec();
    Output::from_result(&result, &dir)
}

/// Run the reference `acap-build` in a sub-process.
///
/// It must be on the path on the `PATH`.
pub fn build_with_reference(cli: Cli) -> anyhow::Result<Output> {
    let Cli {
        path,
        build,
        manifest,
        additional_file,
        disable_manifest_validation,
        oecore_target_arch,
        axis_os_version,
        acap_sdk_location: _,
        source_date_epoch,
        acap_build_impl: _,
    } = cli;

    let mut command = Command::new("acap-build");

    // Environment variables
    command.env(
        "OECORE_TARGET_ARCH",
        oecore_target_arch
            .to_possible_value()
            .expect("no architecture variant is skipped")
            .get_name(),
    );
    match &axis_os_version {
        Some(version) => command.env("AXIS_OS_VERSION", version),
        None => command.env_remove("AXIS_OS_VERSION"),
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

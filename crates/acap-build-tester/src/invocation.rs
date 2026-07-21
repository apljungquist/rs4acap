use std::{env, process::Command};

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
        oecore_native_sysroot,
        sdk_target_sysroot,
        acap_sdk_location: _,
        source_date_epoch,
        acap_build_impl: _,
    } = cli;

    let mut command = Command::new("acap-build");

    // Start from an empty environment and populate only what the reference
    // implementation needs, so that stray variables in the caller's environment
    // (e.g. `RUST_LOG`, or the `LD_LIBRARY_PATH` that cargo sets for the
    // programs it runs) cannot make the reference build diverge from the
    // candidate.
    command.env_clear();

    // `PATH` locates `acap-build` and the tools it shells out to
    // (`manifest-generator`, `eap-create.sh`, `tar`, ...). It is inherited
    // rather than modelled because it is not part of the invocation under test.
    if let Some(path) = env::var_os("PATH") {
        command.env("PATH", path);
    }

    // Set from the invocation rather than inherited, so the fuzzer controls them.
    command.env(
        "OECORE_TARGET_ARCH",
        oecore_target_arch
            .to_possible_value()
            .expect("no architecture variant is skipped")
            .get_name(),
    );
    // `eap-create.sh` reads these to locate its config and derive the package
    // architecture.
    command.env("OECORE_NATIVE_SYSROOT", oecore_native_sysroot);
    command.env("SDKTARGETSYSROOT", sdk_target_sysroot);
    if let Some(epoch) = source_date_epoch {
        command.env("SOURCE_DATE_EPOCH", u64::from(epoch).to_string());
    }

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
        .output()?;
    Output::from_command(output, &path)
}

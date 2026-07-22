//! Conservative-mode checks: refuse inputs for which the correct behavior is ambiguous

use std::{ffi::OsStr, path::Path};

use rs4a_eap::Architecture;

use crate::{Cli, OpenEmbeddedTargetArchitecture};

/// A rejection by conservative mode, carrying a human-readable explanation.
#[derive(Debug, thiserror::Error)]
#[error("conservative mode: {0}")]
pub struct ConservativeRejection(String);

macro_rules! reject {
    ($($arg:tt)*) => {
        return Err(ConservativeRejection(format!($($arg)*)))
    };
}

/// Return an error if the correct behavior is ambiguous
pub fn error_for_rejection(cli: &Cli) -> Result<(), ConservativeRejection> {
    let Cli {
        oecore_native_sysroot,
        oecore_target_arch,
        sdk_target_sysroot,
        ..
    } = cli;

    // Ordered cheapest-first
    error_for_unset_native_sysroot(oecore_native_sysroot.as_deref())?;
    let sdk_target_sysroot = error_for_unset_target_sysroot(sdk_target_sysroot.as_deref())?;
    error_for_inconsistent_architecture(sdk_target_sysroot, *oecore_target_arch)?;
    Ok(())
}

/// The reference implementation depends on `OECORE_NATIVE_SYSROOT` being set correctly to
/// succeed and/or produce correct output.
/// Either way, no reliable reference output exists and the desired output is therefore ambiguous.
fn error_for_unset_native_sysroot(value: Option<&Path>) -> Result<(), ConservativeRejection> {
    if non_empty(value).is_none() {
        reject!("OECORE_NATIVE_SYSROOT is not set");
    }
    Ok(())
}

/// The reference implementation infers the package architecture from `SDKTARGETSYSROOT` and aborts
/// when it is unset.
/// Since no reference output exists for such invocations, the desired output is ambiguous.
fn error_for_unset_target_sysroot(value: Option<&Path>) -> Result<&Path, ConservativeRejection> {
    let Some(value) = non_empty(value) else {
        reject!("SDKTARGETSYSROOT is not set");
    };
    Ok(value)
}

/// The reference implementation sets a default package architecture in manifest.json and
/// package.conf, but it infers them from different environment variables.
/// In the reference environment these are configured consistently,
/// but when they are not the expected output is ambiguous.
fn error_for_inconsistent_architecture(
    target_sysroot: &Path,
    target_arch: OpenEmbeddedTargetArchitecture,
) -> Result<(), ConservativeRejection> {
    let expected = Architecture::from(target_arch);
    let derived = architecture_from_sysroot(target_sysroot)?;
    if derived != expected {
        reject!("SDKTARGETSYSROOT and OECORE_TARGET_ARCH imply different architectures");
    }
    Ok(())
}

/// Infer the [`Architecture`] from `SDKTARGETSYSROOT`
fn architecture_from_sysroot(
    sdk_target_sysroot: &Path,
) -> Result<Architecture, ConservativeRejection> {
    let Some(leaf) = sdk_target_sysroot.file_name().and_then(OsStr::to_str) else {
        reject!(
            "SDKTARGETSYSROOT={sdk_target_sysroot:?} has no final path component or is not UTF-8"
        );
    };
    let machine = leaf
        .split_once("-poky-linux")
        .map_or(leaf, |(before, _)| before);
    Ok(match machine {
        "cortexa9hf-neon" | "armv7hf" => Architecture::Armv7hf,
        "cortexa53-crypto" | "aarch64" => Architecture::Aarch64,
        other => reject!(
            "SDKTARGETSYSROOT names architecture {other:?}, which the reference does not recognise"
        ),
    })
}

fn non_empty(value: Option<&Path>) -> Option<&Path> {
    value.filter(|v| !v.as_os_str().is_empty())
}

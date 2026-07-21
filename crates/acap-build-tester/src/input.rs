//! The input that affects the execution of an `acap-build` implementation.

use std::path::PathBuf;

use acap_build::{Architecture, BuildOption, Cli};
use proptest::{
    arbitrary::any,
    prelude::{BoxedStrategy, Just, Strategy},
    prop_oneof,
};
use rs4a_eap::{Mtime, DEFAULT_ACAP_SDK_LOCATION};

use crate::source::Source;

/// The complete, known input to an `acap-build` implementation.
#[derive(Clone, Debug)]
pub struct Input {
    pub source: Source,
    pub invocation: Cli,
}

pub fn arbitrary_input(
    oecore_target_arch: Architecture,
    sdk_target_sysroot: Option<PathBuf>,
) -> BoxedStrategy<Input> {
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
                // Only the default is generated: the reference does not read it, so any other
                // location would make only the candidate use different schema which is an
                // unnecessary potential source of divergence.
                sdk_target_sysroot: sdk_target_sysroot.clone(),
                acap_sdk_location: PathBuf::from(DEFAULT_ACAP_SDK_LOCATION),
                // Always set: `None` falls back to the current time, which the two
                // implementations would sample at different moments.
                source_date_epoch: Some(
                    Mtime::try_from(epoch).expect("generated values fit in the tar headers"),
                ),
            },
            source,
        })
        .boxed()
}

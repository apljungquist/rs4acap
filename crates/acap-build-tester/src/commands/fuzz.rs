use acap_build::{Cli, OpenEmbeddedTargetArchitecture};
use anyhow::{bail, Context};
use proptest::test_runner::{Config, RngAlgorithm, TestCaseError, TestError, TestRng, TestRunner};

use crate::{
    input::{arbitrary_input, Input},
    invocation::{build_with_candidate, build_with_reference},
};

fn check(input: &Input) -> anyhow::Result<()> {
    let candidate_dir = tempfile::tempdir()?;
    input.source.materialize_in(candidate_dir.path())?;
    let candidate = build_with_candidate(Cli {
        path: candidate_dir.path().to_path_buf(),
        ..input.invocation.clone()
    })
    .context("building with the candidate")?;

    let reference_dir = tempfile::tempdir()?;
    input.source.materialize_in(reference_dir.path())?;
    let reference = build_with_reference(Cli {
        path: reference_dir.path().to_path_buf(),
        ..input.invocation.clone()
    })
    .context("building with the reference")?;

    if candidate.essence() != reference.essence() {
        bail!("the candidate does not match the reference:\n{candidate:#?}\n{reference:#?}");
    }
    Ok(())
}

fn fuzz(
    oecore_target_arch: OpenEmbeddedTargetArchitecture,
    cases: u32,
    seed: u64,
) -> Result<(), Box<TestError<Input>>> {
    let mut rng_seed = [0u8; 32];
    for (dst, src) in rng_seed.iter_mut().zip(seed.to_le_bytes()) {
        *dst = src;
    }

    let config = Config {
        cases,
        failure_persistence: None,
        ..Config::default()
    };
    let rng = TestRng::from_seed(RngAlgorithm::ChaCha, &rng_seed);

    TestRunner::new_with_rng(config, rng)
        .run(&arbitrary_input(oecore_target_arch), |input| {
            check(&input).map_err(|e| TestCaseError::fail(format!("{e:#}")))
        })
        .map_err(Box::new)
}

#[derive(clap::Parser)]
pub struct FuzzCommand {
    /// The architecture to build for.
    #[clap(long, env = "OECORE_TARGET_ARCH")]
    oecore_target_arch: OpenEmbeddedTargetArchitecture,
    /// Number of random inputs to try.
    #[clap(long, env = "ACAP_BUILD_FUZZ_CASES", default_value_t = 1)]
    cases: u32,
    /// Seed for the random number generator.
    #[clap(long, env = "ACAP_BUILD_FUZZ_SEED", default_value_t = 0)]
    seed: u64,
}

impl FuzzCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self {
            oecore_target_arch,
            cases,
            seed,
        } = self;

        match fuzz(oecore_target_arch, cases, seed).map_err(|e| *e) {
            Ok(()) => Ok(()),
            Err(TestError::Fail(reason, input)) => {
                bail!("Property violated by {input:#?}:\n{reason}")
            }
            Err(e @ TestError::Abort(_)) => bail!("Fuzzing aborted: {e}"),
        }
    }
}

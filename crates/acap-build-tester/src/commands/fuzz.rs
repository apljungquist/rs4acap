use std::sync::atomic::{AtomicU64, Ordering};

use acap_build::{Cli, OpenEmbeddedTargetArchitecture};
use anyhow::{bail, Context};
use proptest::test_runner::{Config, RngAlgorithm, TestCaseError, TestError, TestRng, TestRunner};

use crate::{
    input::{arbitrary_input, Input},
    invocation::{build_with_candidate, build_with_reference},
};

/// The outcome of comparing the candidate against the reference on one input.
enum Comparison {
    /// The candidate succeeded and produced the same essence as the reference.
    Matched,
    /// The candidate declined the input, so there was nothing to compare against.
    ///
    /// Conservative mode is allowed to fail, so this is not a property violation. It is reported
    /// separately so that a candidate which "passes" by declining every input does not go unnoticed.
    Declined,
}

fn check(input: &Input) -> anyhow::Result<Comparison> {
    let candidate_dir = tempfile::tempdir()?;
    input.source.materialize_in(candidate_dir.path())?;
    let candidate = build_with_candidate(Cli {
        path: candidate_dir.path().to_path_buf(),
        ..input.invocation.clone()
    })
    .context("building with the candidate")?;

    // This does not distinguish between inputs rejected by the conservative mode and genuine
    // crashes.
    // TODO: Consider distinguishing between failure modes in `acap-build`
    if !candidate.essence().success {
        return Ok(Comparison::Declined);
    }

    let reference_dir = tempfile::tempdir()?;
    input.source.materialize_in(reference_dir.path())?;
    let reference = build_with_reference(Cli {
        path: reference_dir.path().to_path_buf(),
        ..input.invocation.clone()
    })
    .context("building with the reference")?;

    if candidate.essence() != reference.essence() {
        bail!("the candidate succeeded but does not match the reference:\n{candidate:#?}\n{reference:#?}");
    }
    Ok(Comparison::Matched)
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
        // Accept a rejection rate of 80%
        // TODO: Consider tuning this value and/or making it configurable
        max_global_rejects: 4 * cases,
        ..Config::default()
    };
    let rng = TestRng::from_seed(RngAlgorithm::ChaCha, &rng_seed);

    let matched = AtomicU64::new(0);
    let declined = AtomicU64::new(0);

    let result = TestRunner::new_with_rng(config, rng)
        .run(&arbitrary_input(oecore_target_arch), |input| {
            match check(&input).map_err(|e| TestCaseError::fail(format!("{e:#}")))? {
                Comparison::Matched => {
                    matched.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
                Comparison::Declined => {
                    declined.fetch_add(1, Ordering::Relaxed);
                    Err(TestCaseError::reject("the candidate declined the input"))
                }
            }
        })
        .map_err(Box::new);

    let matched = matched.load(Ordering::Relaxed);
    let declined = declined.load(Ordering::Relaxed);
    let total = matched + declined;
    if total != 0 {
        log::info!(
            "The candidate matched the reference on {matched} and declined {declined} of {total} inputs ({percent:.1}% declined).",
            percent = 100.0 * declined as f64 / total as f64,
        );
    }

    result
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

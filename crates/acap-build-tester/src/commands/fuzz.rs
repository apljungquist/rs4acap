use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use acap_build::Cli;
use anyhow::{bail, Context};
use proptest::test_runner::{Config, RngAlgorithm, TestCaseError, TestError, TestRng, TestRunner};

use crate::{
    input::{arbitrary_input, Input},
    invocation::{build_with, Environment},
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

fn check(candidate_exe: &Path, input: &Input) -> anyhow::Result<Comparison> {
    let candidate_dir = tempfile::tempdir()?;
    input.source.materialize_in(candidate_dir.path())?;
    let candidate = build_with(
        candidate_exe,
        Cli {
            path: candidate_dir.path().to_path_buf(),
            ..input.invocation.clone()
        },
    )
    .context("building with the candidate")?;

    // This does not distinguish between inputs rejected by the conservative mode and genuine
    // crashes.
    // TODO: Consider distinguishing between failure modes in `acap-build`
    if !candidate.essence().success {
        return Ok(Comparison::Declined);
    }

    let reference_dir = tempfile::tempdir()?;
    input.source.materialize_in(reference_dir.path())?;
    let reference = build_with(
        "acap-build",
        Cli {
            path: reference_dir.path().to_path_buf(),
            ..input.invocation.clone()
        },
    )
    .context("building with the reference")?;

    if candidate.essence() != reference.essence() {
        bail!("the candidate succeeded but does not match the reference:\n{candidate:#?}\n{reference:#?}");
    }
    Ok(Comparison::Matched)
}

fn fuzz(
    candidate: &Path,
    environment: Environment,
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
        .run(&arbitrary_input(environment), |input| {
            match check(candidate, &input).map_err(|e| TestCaseError::fail(format!("{e:#}")))? {
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
    /// Number of random inputs to try.
    #[clap(long, env = "ACAP_BUILD_FUZZ_CASES", default_value_t = 1)]
    cases: u32,
    /// Seed for the random number generator.
    #[clap(long, env = "ACAP_BUILD_FUZZ_SEED", default_value_t = 0)]
    seed: u64,
    #[clap(flatten)]
    environment: Environment,
}

impl FuzzCommand {
    pub fn exec(self, candidate: &Path) -> anyhow::Result<()> {
        let Self {
            cases,
            seed,
            environment,
        } = self;

        match fuzz(candidate, environment, cases, seed).map_err(|e| *e) {
            Ok(()) => Ok(()),
            Err(TestError::Fail(reason, input)) => {
                bail!("Property violated by {input:#?}:\n{reason}")
            }
            Err(e @ TestError::Abort(_)) => bail!("Fuzzing aborted: {e}"),
        }
    }
}

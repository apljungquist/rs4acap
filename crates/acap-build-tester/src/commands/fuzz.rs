use std::{
    path::Path,
    sync::atomic::{AtomicU32, Ordering},
};

use anyhow::{bail, Context};
use proptest::test_runner::{Config, RngAlgorithm, TestCaseError, TestError, TestRng, TestRunner};

use crate::{
    input::{arbitrary_input, Input},
    invocation::{build_with, Environment},
};

fn check(candidate_exe: &Path, input: &Input) -> anyhow::Result<()> {
    let candidate_dir = tempfile::tempdir()?;
    input.source.materialize_in(candidate_dir.path())?;
    let candidate = build_with(
        candidate_exe,
        candidate_dir.path(),
        input.invocation.clone(),
    )
    .context("building with the candidate")?;

    let reference_dir = tempfile::tempdir()?;
    input.source.materialize_in(reference_dir.path())?;
    let reference = build_with("acap-build", reference_dir.path(), input.invocation.clone())
        .context("building with the reference")?;

    if candidate.essence() != reference.essence() {
        bail!("the candidate succeeded but does not match the reference:\n{candidate:#?}\n{reference:#?}");
    }
    Ok(())
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

    let compared = AtomicU32::new(0);
    let rejected = AtomicU32::new(0);

    let result = TestRunner::new_with_rng(config, rng)
        .run(&arbitrary_input(environment), |input| {
            match input.invocation.error_for_conservative() {
                Ok(()) => {
                    compared.fetch_add(1, Ordering::Relaxed);
                    check(candidate, &input).map_err(|e| TestCaseError::fail(format!("{e:#}")))
                }
                Err(e) => {
                    rejected.fetch_add(1, Ordering::Relaxed);
                    Err(TestCaseError::reject(e.to_string()))
                }
            }
        })
        .map_err(Box::new);

    let compared: f64 = compared.load(Ordering::Relaxed).into();
    let rejected: f64 = rejected.load(Ordering::Relaxed).into();
    let total = compared + rejected;
    if total != 0.0 {
        log::info!(
            "The candidate was compared to the reference on {compared} and rejected {rejected} of {total} inputs ({percent:.1}% rejected).",
            percent = 100.0 * rejected / total
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

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use acap_build::Cli;
use anyhow::{bail, Context};
use clap::ValueEnum;
use proptest::test_runner::{Config, RngAlgorithm, TestCaseError, TestError, TestRng, TestRunner};

use crate::{
    input::{arbitrary_input, Input},
    invocation::{build_with_candidate, build_with_reference, Environment},
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

fn fuzz(environment: Environment, cases: u32, seed: u64) -> Result<(), Box<TestError<Input>>> {
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

/// Materialize a failing input as a replayable example.
///
/// The source is written into `example_dir` and the invocation into the sibling
/// `invocations/<example>` tree that `replay` reads, mirroring how the examples are laid out.
fn save_example(example_dir: &Path, input: &Input) -> anyhow::Result<PathBuf> {
    input.source.materialize_in(example_dir)?;

    let name = example_dir
        .file_name()
        .context("--save-failing path has no final component")?;
    let invocations_dir = example_dir
        .parent()
        .context("--save-failing path has no parent")?
        .with_file_name("invocations")
        .join(name);
    fs::create_dir_all(&invocations_dir)?;

    // The stem names the replay trial and keeps sibling invocations distinct. The architecture is
    // the only thing that varies between the reference environments and is what replay gates on, so
    // it is the natural, collision-free key.
    let arch = input
        .invocation
        .oecore_target_arch
        .to_possible_value()
        .expect("every architecture variant has a name");
    let path = invocations_dir.join(format!("{}.json", arch.get_name()));

    // Record the whole invocation, but with a `path` relative to the example directory so that
    // replay can resolve it against a scratch copy.
    let invocation = Cli {
        path: PathBuf::from("."),
        ..input.invocation.clone()
    };
    let mut json = serde_json::to_string_pretty(&invocation)?;
    json.push('\n');
    fs::write(&path, json)?;
    Ok(path)
}

#[derive(clap::Parser)]
pub struct FuzzCommand {
    /// Number of random inputs to try.
    #[clap(long, env = "ACAP_BUILD_FUZZ_CASES", default_value_t = 1)]
    cases: u32,
    /// Seed for the random number generator.
    #[clap(long, env = "ACAP_BUILD_FUZZ_SEED", default_value_t = 0)]
    seed: u64,
    /// Directory in which to record the shrunk failing input as an example, if any.
    #[clap(long)]
    save_failing: Option<PathBuf>,
    #[clap(flatten)]
    environment: Environment,
}

impl FuzzCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self {
            cases,
            seed,
            save_failing,
            environment,
        } = self;

        match fuzz(environment, cases, seed).map_err(|e| *e) {
            Ok(()) => Ok(()),
            Err(TestError::Fail(reason, input)) => {
                let saved = match &save_failing {
                    Some(dir) => {
                        let path = save_example(dir, &input).context("saving the failing input")?;
                        format!("\nSaved failing example to {path:?}")
                    }
                    None => String::new(),
                };
                bail!("Property violated by {input:#?}:\n{reason}{saved}")
            }
            Err(e @ TestError::Abort(_)) => bail!("Fuzzing aborted: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use acap_build::{BuildOption, OpenEmbeddedTargetArchitecture};
    use rs4a_eap::{AcapBuildImpl, Mtime};

    use super::*;
    use crate::source::{Manifest, Source, DEFAULT_MANIFEST_NAME};

    fn sample_invocation() -> Cli {
        Cli {
            path: PathBuf::from("."),
            build: BuildOption::NoBuild,
            manifest: PathBuf::from("manifest.json"),
            additional_file: vec![PathBuf::from("extra.txt")],
            disable_manifest_validation: true,
            oecore_target_arch: OpenEmbeddedTargetArchitecture::Arm,
            oecore_native_sysroot: Some(PathBuf::from("/native")),
            sdk_target_sysroot: Some(PathBuf::from("/target/armv7hf")),
            acap_sdk_location: PathBuf::from("/opt/axis/"),
            source_date_epoch: Some(Mtime::try_from(0).unwrap()),
            acap_build_impl: AcapBuildImpl::Equivalent,
            conservative: false,
        }
    }

    #[test]
    fn recorded_invocation_round_trips() {
        let cli = sample_invocation();

        let json = serde_json::to_string_pretty(&cli).unwrap();
        // Lock the on-disk spellings the recorded examples depend on.
        assert!(json.contains("\"no-build\""), "{json}");
        assert!(json.contains("\"arm\""), "{json}");
        assert!(json.contains("\"equivalent\""), "{json}");

        let back: Cli = serde_json::from_str(&json).unwrap();
        assert_eq!(cli, back);
    }

    #[test]
    fn save_example_writes_replayable_files() {
        let tmp = tempfile::tempdir().unwrap();
        let example_dir = tmp.path().join("data").join("myapp");
        let input = Input {
            source: Source {
                manifest: Manifest {
                    schema_version: "1.3",
                    app_name: "myapp".to_string(),
                    version: "1.0.0".to_string(),
                    friendly_name: None,
                },
                manifest_name: DEFAULT_MANIFEST_NAME.to_string(),
                additional_files: BTreeSet::new(),
                html: false,
            },
            invocation: sample_invocation(),
        };

        let path = save_example(&example_dir, &input).unwrap();

        // The source lands in the example dir; the invocation in the sibling `invocations` tree,
        // stemmed by architecture, exactly where `replay` looks for it.
        assert!(example_dir.join("manifest.json").exists());
        assert_eq!(
            path,
            tmp.path().join("invocations").join("myapp").join("arm.json")
        );

        // The recorded invocation deserializes back with its `path` relative to the example dir.
        let recorded: Cli =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(recorded, input.invocation);
    }
}

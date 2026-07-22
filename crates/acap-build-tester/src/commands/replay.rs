use std::{
    fs,
    path::{Path, PathBuf},
};

use acap_build::{BuildOption, Cli, DEFAULT_ACAP_SDK_LOCATION};
use anyhow::{bail, ensure, Context};
use libtest_mimic::{Arguments, Failed, Trial};
use rs4a_eap::{AcapBuildImpl, Mtime};

use crate::invocation::{build_with_candidate, build_with_reference, Environment};

fn copy_dir(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            // `fs::copy` preserves the mode, which the executable and scripts rely on.
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Copy the app in `app_dir` into a scratch directory so that the generated files don't pollute
/// the app dir.
fn scratch_copy(app_dir: &Path) -> anyhow::Result<(tempfile::TempDir, PathBuf)> {
    let name = app_dir.file_name().context("app dir has no name")?;
    let scratch = tempfile::tempdir()?;
    let app = scratch.path().join(name);
    copy_dir(app_dir, &app)?;
    Ok((scratch, app))
}

fn check(app_dir: PathBuf, environment: &Environment) -> anyhow::Result<()> {
    let (_candidate_scratch, candidate_app) = scratch_copy(&app_dir)?;
    let (_reference_scratch, reference_app) = scratch_copy(&app_dir)?;

    // Both implementations receive the same inputs, except for the directory they build in.
    let cli = Cli {
        path: candidate_app,
        build: BuildOption::NoBuild,
        manifest: PathBuf::from("manifest.json"),
        // TODO: Enable storing arguments alongside examples
        additional_file: Vec::new(),
        disable_manifest_validation: true,
        oecore_target_arch: environment.oecore_target_arch,
        oecore_native_sysroot: environment.oecore_native_sysroot.clone(),
        sdk_target_sysroot: environment.sdk_target_sysroot.clone(),
        acap_sdk_location: PathBuf::from(DEFAULT_ACAP_SDK_LOCATION),
        source_date_epoch: Some(Mtime::default()),
        acap_build_impl: AcapBuildImpl::Equivalent,
        conservative: false,
    };
    let candidate = build_with_candidate(cli.clone()).context("building with the candidate")?;
    let reference = build_with_reference(Cli {
        path: reference_app,
        ..cli
    })
    .context("building with the reference")?;

    if candidate.essence() != reference.essence() {
        bail!("the candidate does not match the reference:\n{candidate:#?}\n{reference:#?}");
    }

    if !candidate.status.success() {
        bail!("the example failed to build: \n{candidate:#?}\n{reference:#?}");
    }

    Ok(())
}

#[derive(clap::Parser)]
pub struct ReplayCommand {
    #[clap(flatten)]
    environment: Environment,
    /// Directory containing the source code of one application per subdirectory.
    apps: PathBuf,
    #[clap(flatten)]
    test_args: Arguments,
}

impl ReplayCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self {
            environment,
            apps,
            test_args,
        } = self;

        let mut trials = Vec::new();
        for entry in fs::read_dir(&apps).with_context(|| format!("reading {apps:?}"))? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let app = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                let environment = environment.clone();
                trials.push(Trial::test(name, move || {
                    check(app, &environment).map_err(|e| Failed::from(format!("{e:#}")))
                }));
            }
        }
        trials.sort_by(|a, b| a.name().cmp(b.name()));
        ensure!(!trials.is_empty(), "found no apps in {apps:?}");

        libtest_mimic::run(&test_args, trials).exit_if_failed();
        Ok(())
    }
}

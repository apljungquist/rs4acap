use std::{
    fs,
    path::{Path, PathBuf},
};

use acap_build::{Architecture, Cli};
use anyhow::{bail, ensure, Context};
use libtest_mimic::{Arguments, Failed, Trial};

use crate::{
    input::load_invocation,
    invocation::{build_with_candidate, build_with_reference},
};

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

fn check(
    app_dir: PathBuf,
    oecore_target_arch: Architecture,
    axis_os_version: Option<String>,
) -> anyhow::Result<()> {
    let (_candidate_scratch, candidate_app) = scratch_copy(&app_dir)?;
    let (_reference_scratch, reference_app) = scratch_copy(&app_dir)?;

    // Both implementations receive the same inputs, except for the directory they build in. The
    // invocation is read from the example itself, falling back to the ambient environment.
    let cli = load_invocation(&app_dir, candidate_app, oecore_target_arch, axis_os_version)?;
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
    #[clap(long, env = "OECORE_TARGET_ARCH")]
    oecore_target_arch: Architecture,
    /// Fallback AXIS OS version for examples that do not record their own.
    #[clap(long, env = "AXIS_OS_VERSION")]
    axis_os_version: Option<String>,
    /// Directory containing the source code of one application per subdirectory.
    apps: PathBuf,
    #[clap(flatten)]
    test_args: Arguments,
}

impl ReplayCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self {
            oecore_target_arch,
            axis_os_version,
            apps,
            test_args,
        } = self;

        let mut trials = Vec::new();
        for entry in fs::read_dir(&apps).with_context(|| format!("reading {apps:?}"))? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let app = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                let axis_os_version = axis_os_version.clone();
                trials.push(Trial::test(name, move || {
                    check(app, oecore_target_arch, axis_os_version)
                        .map_err(|e| Failed::from(format!("{e:#}")))
                }));
            }
        }
        trials.sort_by(|a, b| a.name().cmp(b.name()));
        ensure!(!trials.is_empty(), "found no apps in {apps:?}");

        libtest_mimic::run(&test_args, trials).exit_if_failed();
        Ok(())
    }
}

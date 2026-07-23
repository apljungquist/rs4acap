use std::{
    fs,
    path::{Path, PathBuf},
};

use acap_build::Cli;
use anyhow::{bail, ensure, Context};
use libtest_mimic::{Arguments, Failed, Trial};

use crate::invocation::{build_with_candidate, build_with_reference};

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

/// Read a recorded invocation, i.e. a [`Cli`] serialized to JSON.
fn read_invocation(path: &Path) -> anyhow::Result<Cli> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {path:?}"))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {path:?}"))
}

fn check(app_dir: &Path, invocation: &Cli) -> anyhow::Result<()> {
    let (_candidate_scratch, candidate_app) = scratch_copy(app_dir)?;
    let (_reference_scratch, reference_app) = scratch_copy(app_dir)?;

    // The recorded `path` is only a placeholder; override it so each implementation builds in a
    // scratch copy of its own and cannot see the other's output.
    let candidate = build_with_candidate(Cli {
        path: candidate_app,
        ..invocation.clone()
    })
    .context("building with the candidate")?;
    let reference = build_with_reference(Cli {
        path: reference_app,
        ..invocation.clone()
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

/// The recorded invocations of one app, read from its `invocations/<app>` directory.
fn invocation_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {dir:?} (every app must have recorded invocations)"))?
        .map(|entry| Ok(entry?.path()))
        .collect::<anyhow::Result<_>>()?;
    files.retain(|p| p.extension().is_some_and(|e| e == "json"));
    files.sort();
    ensure!(!files.is_empty(), "found no invocations in {dir:?}");
    Ok(files)
}

#[derive(clap::Parser)]
pub struct ReplayCommand {
    /// Directory containing the source code of one application per subdirectory.
    ///
    /// Each app's recorded invocations are read from the sibling `invocations/<app>` directory.
    apps: PathBuf,
    #[clap(flatten)]
    test_args: Arguments,
}

impl ReplayCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self { apps, test_args } = self;

        let invocations_root = apps.with_file_name("invocations");

        let mut trials = Vec::new();
        for entry in fs::read_dir(&apps).with_context(|| format!("reading {apps:?}"))? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let app = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();

            for file in invocation_files(&invocations_root.join(&name))? {
                let invocation = read_invocation(&file)?;
                let stem = file.file_stem().unwrap_or_default().to_string_lossy();
                let trial_name = format!("{name}::{stem}");
                // The reference derives the architecture and locates its SDK from the environment,
                // so an invocation recorded elsewhere cannot be replayed here.

                let app = app.clone();
                trials.push(Trial::test(trial_name, move || {
                    check(&app, &invocation).map_err(|e| Failed::from(format!("{e:#}")))
                }));
            }
        }
        trials.sort_by(|a, b| a.name().cmp(b.name()));
        ensure!(!trials.is_empty(), "found no apps in {apps:?}");

        libtest_mimic::run(&test_args, trials).exit_if_failed();
        Ok(())
    }
}

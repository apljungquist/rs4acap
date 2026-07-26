use std::{
    fs,
    path::{Path, PathBuf},
};

use acap_build::{Cli, OpenEmbeddedTargetArchitecture};
use anyhow::{bail, ensure, Context};
use libtest_mimic::{Arguments, Failed, Trial};

use crate::invocation::{build_with, Environment};

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

fn check(candidate_exe: &Path, app_dir: &Path, invocation: &Cli) -> anyhow::Result<()> {
    let (_candidate_scratch, candidate_app) = scratch_copy(app_dir)?;
    let (_reference_scratch, reference_app) = scratch_copy(app_dir)?;

    // Each implementation builds in a scratch copy of its own so it cannot see the other's output.
    // The recorded `path` is a neutral "." placeholder (see `save_example`); the working directory
    // is what places the build in each scratch copy.
    let candidate = build_with(candidate_exe, &candidate_app, invocation.clone())
        .context("building with the candidate")?;
    let reference = build_with("acap-build", &reference_app, invocation.clone())
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

/// The recorded invocations of one app that can run on `arch`, paired with their file.
///
/// Errors if the app has no recorded invocations at all: a source directory whose
/// `invocations/<app>` is missing or empty is orphaned, which is always a hard error. Returns an
/// empty vector when the app has invocations but none for `arch` — a normal skip, since the other
/// architecture's SDK is not installed on this host and its recorded invocation cannot be replayed
/// here.
fn invocations_for_arch(
    dir: &Path,
    arch: OpenEmbeddedTargetArchitecture,
) -> anyhow::Result<Vec<(PathBuf, Cli)>> {
    let mut matching = Vec::new();
    for file in invocation_files(dir)? {
        let invocation = read_invocation(&file)?;
        if invocation.oecore_target_arch == arch {
            matching.push((file, invocation));
        }
    }
    Ok(matching)
}

#[derive(clap::Parser)]
pub struct ReplayCommand {
    /// The host environment; only invocations recorded for its architecture are replayed.
    #[clap(flatten)]
    environment: Environment,
    /// Directory containing the source code of one application per subdirectory.
    ///
    /// Each app's recorded invocations are read from the sibling `invocations/<app>` directory.
    apps: PathBuf,
    #[clap(flatten)]
    test_args: Arguments,
}

impl ReplayCommand {
    pub fn exec(self, candidate: &Path) -> anyhow::Result<()> {
        let Self {
            environment,
            apps,
            test_args,
        } = self;

        let invocations_root = apps.with_file_name("invocations");

        let mut trials = Vec::new();
        let mut saw_app = false;
        for entry in fs::read_dir(&apps).with_context(|| format!("reading {apps:?}"))? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            saw_app = true;
            let app = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();

            for (file, recorded) in invocations_for_arch(
                &invocations_root.join(&name),
                environment.oecore_target_arch,
            )? {
                // The recorded sysroots are absolute paths from the container that produced the
                // example, so they need not exist on this host; the reference resolves against the
                // current environment's instead. Everything else that defines the counterexample
                // is replayed exactly as recorded. The candidate ignores the sysroots entirely.
                let invocation = Cli {
                    oecore_native_sysroot: environment.oecore_native_sysroot.clone(),
                    sdk_target_sysroot: environment.sdk_target_sysroot.clone(),
                    ..recorded
                };
                // Recorded invocations are named `<example>-<arch>-<hash>`; the `<example>-` prefix
                // repeats this app's name, so drop it from the trial label when present.
                let stem = file.file_stem().unwrap_or_default().to_string_lossy();
                let label = stem.strip_prefix(&format!("{name}-")).unwrap_or(&stem);
                let trial_name = format!("{name}::{label}");

                let app = app.clone();
                let candidate = candidate.to_path_buf();
                trials.push(Trial::test(trial_name, move || {
                    check(&candidate, &app, &invocation).map_err(|e| Failed::from(format!("{e:#}")))
                }));
            }
        }
        ensure!(saw_app, "found no apps in {apps:?}");
        // A single app may legitimately have no invocation for this architecture (a skip), but a
        // run that matched nothing across every app compared the candidate against nothing and
        // must not pass silently — every app carries both architectures, so an empty result means
        // a mistargeted host or a corpus that lost its matching invocations.
        ensure!(
            !trials.is_empty(),
            "found apps in {apps:?} but none has a recorded invocation for this host's \
             architecture ({:?}); nothing to replay",
            environment.oecore_target_arch,
        );
        trials.sort_by(|a, b| a.name().cmp(b.name()));

        libtest_mimic::run(&test_args, trials).exit_if_failed();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use acap_build::BuildOption;
    use rs4a_eap::{AcapBuildImpl, Mtime};

    use super::*;

    fn invocation(arch: OpenEmbeddedTargetArchitecture) -> Cli {
        Cli {
            path: PathBuf::from("."),
            build: BuildOption::NoBuild,
            manifest: PathBuf::from("manifest.json"),
            additional_file: Vec::new(),
            disable_manifest_validation: true,
            oecore_target_arch: arch,
            oecore_native_sysroot: None,
            sdk_target_sysroot: None,
            acap_sdk_location: PathBuf::from("/opt/axis/"),
            source_date_epoch: Some(Mtime::default()),
            acap_build_impl: AcapBuildImpl::Equivalent,
            conservative: false,
        }
    }

    fn write_invocation(dir: &Path, name: &str, cli: &Cli) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(name), serde_json::to_string_pretty(cli).unwrap()).unwrap();
    }

    #[test]
    fn orphaned_app_is_a_hard_error() {
        let tmp = tempfile::tempdir().unwrap();
        // A source dir whose `invocations/<app>` directory does not exist at all.
        let missing = tmp.path().join("app");
        assert!(invocations_for_arch(&missing, OpenEmbeddedTargetArchitecture::Aarch64).is_err());

        // An empty `invocations/<app>` directory is orphaned too.
        fs::create_dir_all(&missing).unwrap();
        assert!(invocations_for_arch(&missing, OpenEmbeddedTargetArchitecture::Aarch64).is_err());
    }

    #[test]
    fn only_the_matching_architecture_is_replayed() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("app");
        write_invocation(
            &dir,
            "arm-0.json",
            &invocation(OpenEmbeddedTargetArchitecture::Arm),
        );
        write_invocation(
            &dir,
            "aarch64-0.json",
            &invocation(OpenEmbeddedTargetArchitecture::Aarch64),
        );

        let matching = invocations_for_arch(&dir, OpenEmbeddedTargetArchitecture::Aarch64).unwrap();
        assert_eq!(matching.len(), 1);
        assert_eq!(
            matching[0].1.oecore_target_arch,
            OpenEmbeddedTargetArchitecture::Aarch64
        );
    }

    #[test]
    fn invocations_only_for_another_architecture_are_skipped_not_failed() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("app");
        write_invocation(
            &dir,
            "arm-0.json",
            &invocation(OpenEmbeddedTargetArchitecture::Arm),
        );

        // The app has invocations, just none for this host's architecture: an empty result rather
        // than an error, so replay skips it instead of failing.
        let matching = invocations_for_arch(&dir, OpenEmbeddedTargetArchitecture::Aarch64).unwrap();
        assert!(matching.is_empty());
    }
}

//! Archive creation that delegates to the system `tar` and `gzip`.
//!
//! With matching inputs the output is bit-identical to the upstream `acap-build`, at the cost of
//! requiring a GNU-compatible `tar` (notably, macOS ships BSD `tar`) and `gzip` on the `PATH`.
//! Bit-exactness depends on the versions of `tar` and `gzip` that are resolved.
use std::{path::Path, process::Command};

use log::{debug, warn};

use crate::{command_utils::RunWith, Mtime};

/// Finds an available GNU `tar`, returning its program name, or `None` if
/// only a non-GNU `tar` (e.g. BSD `tar` on macOS) is installed.
#[cfg(test)]
fn gnu_tar() -> Option<&'static str> {
    ["tar", "gtar"].into_iter().find(|program| {
        Command::new(program)
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .is_some_and(|output| String::from_utf8_lossy(&output.stdout).contains("GNU tar"))
    })
}

pub struct EquivalentArchiveBuilder(Command);

impl EquivalentArchiveBuilder {
    fn init(
        mut self,
        staging_dir: &Path,
        eap_file_name: &str,
        mtime: Mtime,
        use_compression: bool,
    ) -> Self {
        self.0.current_dir(staging_dir);
        self.0
            .args(["--exclude", "*~"])
            .args(["--file", eap_file_name])
            .args(["--format", "gnu"])
            .args(["--group", "0"])
            .args(["--mtime", &format!("@{}", mtime.0)])
            .args(["--owner", "0"])
            .args(["--sort", "name"]);
        if use_compression {
            self.0.args(["--use-compress-program", "gzip --no-name -9"]);
        }
        self.0
            .arg("--create")
            .arg("--numeric-owner")
            .arg("--exclude-vcs");
        self
    }
    pub fn new(staging_dir: &Path, eap_file_name: &str, mtime: Mtime) -> Self {
        Self(Command::new("tar")).init(staging_dir, eap_file_name, mtime, true)
    }

    #[expect(dead_code, reason = "Will be used in the next commit")]
    #[cfg(test)]
    pub fn new_portable_without_compression(
        staging_dir: &Path,
        eap_file_name: &str,
        mtime: Mtime,
    ) -> Option<Self> {
        gnu_tar().map(|p| Self(Command::new(p)).init(staging_dir, eap_file_name, mtime, false))
    }

    pub fn file(&mut self, name: &str) -> &mut Self {
        self.0.arg(name);
        self
    }

    pub fn files(&mut self, files: &[&str]) -> &mut Self {
        self.0.args(files);
        self
    }

    pub fn run_with_logged_output(mut self) -> anyhow::Result<()> {
        self.0.arg("--verbose");
        self.0.run_with_processed_output(
            |line| {
                let line = line?;
                if !line.is_empty() {
                    debug!("Child said {line:?}.");
                }
                Ok(())
            },
            |line| {
                let line = line?;
                if line.starts_with("tar: Option --mtime: Treating date") {
                    debug!("Child said {line:?}.");
                } else if !line.is_empty() {
                    warn!("Child said {line:?}.");
                }
                Ok(())
            },
        )
    }
}

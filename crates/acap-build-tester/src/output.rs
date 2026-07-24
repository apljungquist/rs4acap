//! The observable output of an `acap-build` implementation.

use std::{
    fmt::{Debug, Formatter},
    fs,
    path::{Path, PathBuf},
    process::ExitStatus,
};

use anyhow::Context;

#[derive(Eq, PartialEq)]
pub struct EmbeddedApplicationPackage {
    /// Location relative to the directory the implementation was run in.
    pub rel: PathBuf,
    pub content: Vec<u8>,
}

impl Debug for EmbeddedApplicationPackage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddedApplicationPackage")
            .field("rel", &self.rel)
            .field("content (length)", &self.content.len())
            .finish()
    }
}

fn collect_eaps(
    root: &Path,
    dir: &Path,
    eaps: &mut Vec<EmbeddedApplicationPackage>,
) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_eaps(root, &path, eaps)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("eap") {
            eaps.push(EmbeddedApplicationPackage {
                rel: path
                    .strip_prefix(root)
                    .context("could not compute relative path")?
                    .to_path_buf(),
                content: fs::read(&path)?,
            });
        }
    }
    Ok(())
}

/// The part of the output that implementations must agree on.
#[derive(Debug, Eq, PartialEq)]
pub struct Essence<'a> {
    pub success: bool,
    pub eaps: &'a [EmbeddedApplicationPackage],
}

#[derive(Debug)]
pub struct Output {
    pub status: ExitStatus,
    pub eaps: Vec<EmbeddedApplicationPackage>,
    #[expect(dead_code, reason = "read by the derived `Debug` implementation")]
    pub stdout: String,
    #[expect(dead_code, reason = "read by the derived `Debug` implementation")]
    pub stderr: String,
}

impl Output {
    /// The output of an implementation that ran as a child process in `dir`.
    pub fn from_command(output: std::process::Output, dir: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            status: output.status,
            eaps: Self::eaps(dir)?,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    fn eaps(dir: &Path) -> anyhow::Result<Vec<EmbeddedApplicationPackage>> {
        let mut eaps = Vec::new();
        collect_eaps(dir, dir, &mut eaps)?;
        eaps.sort_by(|a, b| a.rel.cmp(&b.rel));
        Ok(eaps)
    }

    pub fn essence(&self) -> Essence {
        Essence {
            success: self.status.success(),
            eaps: self.eaps.as_slice(),
        }
    }
}

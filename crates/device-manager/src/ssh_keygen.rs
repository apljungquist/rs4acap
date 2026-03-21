//! Rust API for the `ssh-keygen` program.

use anyhow::bail;
use log::debug;

/// Remove a host from the known_hosts file.
pub fn remove_known_host(host: &str) -> anyhow::Result<()> {
    let output = std::process::Command::new("ssh-keygen")
        .arg("-R")
        .arg(host)
        .output()?;

    debug!(
        "Discarding stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    if !output.status.success() {
        bail!(
            "ssh-keygen -R failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

use anyhow::{bail, Context};
use log::{debug, info, warn};
use rs4a_vapix::{
    basic_device_info_1, json_rpc_http::JsonRpcHttp, parameter_management, pwdgrp, system_ready_1,
    ClientBuilder,
};
use semver::Version;

use crate::Netloc;

#[derive(Clone, Debug, clap::Args)]
pub struct InitCommand {
    #[command(flatten)]
    netloc: Netloc,
}

impl InitCommand {
    pub async fn exec(self) -> anyhow::Result<()> {
        require_root_user(&self.netloc)?;
        initialize(&self.netloc).await
    }
}

fn require_root_user(netloc: &Netloc) -> anyhow::Result<()> {
    if netloc.user != "root" {
        bail!(
            "The --user must be 'root' (got '{}'); the initial user is always 'root' \
             because older firmware requires it",
            netloc.user
        );
    }
    Ok(())
}

fn parse_firmware_version(s: &str) -> anyhow::Result<Version> {
    let mut parts = s.splitn(4, '.');
    let major = parts.next().unwrap_or_default().parse()?;
    let minor = parts.next().unwrap_or_default().parse()?;
    let patch = parts.next().unwrap_or_default().parse()?;
    Ok(Version::new(major, minor, patch))
}

async fn allow_unsigned_apps(client: &rs4a_vapix::Client) -> anyhow::Result<()> {
    client
        .get("axis-cgi/applications/config.cgi")?
        .query(&[
            ("action", "set"),
            ("name", "AllowUnsigned"),
            ("value", "true"),
        ])
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

async fn apply_setup_profile(client: &rs4a_vapix::Client) -> anyhow::Result<()> {
    let data = basic_device_info_1::get_all_unrestricted_properties()
        .send(client)
        .await
        .context("Failed to query firmware version")?;
    let version = parse_firmware_version(&data.property_list.version)
        .context("Failed to parse firmware version")?;
    debug!("Detected firmware version: {version}");

    let allows_unsigned_toggle =
        version >= Version::new(11, 2, 0) && version < Version::new(13, 0, 0);

    if allows_unsigned_toggle {
        info!("Allowing unsigned ACAP applications...");
        allow_unsigned_apps(client).await?;
    } else {
        debug!("Skipping AllowUnsigned (not applicable for firmware {version})");
    }

    Ok(())
}

pub async fn initialize(netloc: &Netloc) -> anyhow::Result<()> {
    info!("Initializing device...");

    // Build anonymous client (device is in setup mode, no auth needed)
    let client = ClientBuilder::new(netloc.host.clone())
        .plain_port(netloc.http_port)
        .secure_port(netloc.https_port)
        .with_inner(|b| b.danger_accept_invalid_certs(true))
        .build_with_automatic_scheme()
        .await?;

    let data = system_ready_1::system_ready().send(&client).await?;
    if !data.needsetup {
        bail!("Expected device to be in setup mode, but needsetup is false");
    }

    info!("Adding root user...");
    pwdgrp::add_user(
        &client,
        "root",
        &netloc.pass,
        pwdgrp::Group::Root,
        pwdgrp::Role::AdminOperatorViewerPtz,
    )
    .await?;

    // Build authenticated client (device is now set up with basic auth)
    let client = ClientBuilder::new(netloc.host.clone())
        .plain_port(netloc.http_port)
        .secure_port(netloc.https_port)
        .basic_authentication("root", &netloc.pass)
        .with_inner(|b| b.danger_accept_invalid_certs(true))
        .build_with_automatic_scheme()
        .await?;

    info!("Enabling SSH...");
    parameter_management::update()
        .set("root.Network.SSH.Enabled", "yes")
        .send(&client)
        .await?;

    info!("Removing device from known_hosts...");
    let host_str = netloc.host.to_string();
    match std::process::Command::new("ssh-keygen")
        .arg("-R")
        .arg(&host_str)
        .output()
    {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            warn!(
                "ssh-keygen -R failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(e) => {
            warn!("Could not run ssh-keygen: {e}");
        }
    }

    apply_setup_profile(&client).await?;

    info!("Device initialized");
    Ok(())
}

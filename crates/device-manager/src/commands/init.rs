use anyhow::{bail, Context};
use log::{debug, info, warn};
use rs4a_vapix::{
    applications_config, basic_device_info_1::GetAllUnrestrictedPropertiesRequest,
    parameter_management, pwdgrp, pwdgrp::AddUserRequest, system_ready_1::SystemReadyRequest,
};
use semver::Version;

use crate::Netloc;

#[derive(Clone, Debug, clap::Args)]
pub struct InitCommand {
    #[command(flatten)]
    pub netloc: Netloc,
}

impl InitCommand {
    pub async fn exec(self) -> anyhow::Result<String> {
        require_root_user(&self.netloc)?;
        initialize(&self.netloc).await?;
        Ok(String::new())
    }
}

pub fn require_root_user(netloc: &Netloc) -> anyhow::Result<()> {
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

async fn apply_setup_profile(client: &rs4a_vapix::Client) -> anyhow::Result<()> {
    let data = GetAllUnrestrictedPropertiesRequest::new()
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
        applications_config::ApplicationConfigRequest::allow_unsigned(true)
            .send(client)
            .await?;
    } else {
        debug!("Skipping AllowUnsigned (not applicable for firmware {version})");
    }

    Ok(())
}

pub async fn initialize(netloc: &Netloc) -> anyhow::Result<()> {
    info!("Initializing device...");

    // Device needs setup, no authentication possible or required
    let client = netloc.connect_anonymous().await?;

    let data = SystemReadyRequest::new().send(&client).await?;
    if !data.needsetup {
        bail!("Expected device to be in setup mode, but needsetup is false");
    }

    info!("Adding root user...");
    AddUserRequest::new(
        "root",
        &netloc.pass,
        pwdgrp::Group::Root,
        pwdgrp::Role::AdminOperatorViewerPtz,
    )
    .send(&client)
    .await?;

    // Device no longer needs setup, authentication is required
    let client = netloc.connect().await?;

    info!("Enabling SSH...");
    parameter_management::UpdateRequest::default()
        .network_ssh_enabled(true)
        .send(&client)
        .await?;

    info!("Removing device from known_hosts...");
    match crate::ssh_keygen::remove_known_host(&netloc.host.to_string()) {
        Ok(()) => {}
        Err(e) => warn!("Failed to remove known_hosts entry: {e:?}"),
    }

    apply_setup_profile(&client).await?;

    info!("Device initialized");
    Ok(())
}

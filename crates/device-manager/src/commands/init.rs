use anyhow::{bail, Context};
use log::{debug, info, warn};
use rs4a_vapix::{
    applications_config, basic_device_info_1, json_rpc_http::JsonRpcHttp, parameter_management,
    pwdgrp, pwdgrp::AddUserRequest, system_ready_1,
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
        initialize(&self.netloc).await
    }
}

fn parse_firmware_version(s: &str) -> anyhow::Result<Version> {
    let mut parts = s.splitn(4, '.');
    let major = parts.next().unwrap_or_default().parse()?;
    let minor = parts.next().unwrap_or_default().parse()?;
    let patch = parts.next().unwrap_or_default().parse()?;
    Ok(Version::new(major, minor, patch))
}

async fn apply_setup_profile(client: &rs4a_vapix::Client, version: &Version) -> anyhow::Result<()> {
    let allows_unsigned_toggle =
        *version >= Version::new(11, 2, 0) && *version < Version::new(13, 0, 0);

    if allows_unsigned_toggle {
        info!("Allowing unsigned ACAP applications...");
        applications_config::ApplicationConfigRequest::allow_unsigned(true)
            .send(client, None)
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

    let data = system_ready_1::system_ready().send(&client).await?;
    if !data.needsetup {
        bail!("Expected device to be in setup mode, but needsetup is false");
    }

    let data = basic_device_info_1::get_all_unrestricted_properties()
        .send(&client)
        .await
        .context("Failed to query firmware version")?;
    let version = parse_firmware_version(&data.property_list.version)
        .context("Failed to parse firmware version")?;
    debug!("Detected firmware version: {version}");

    let needs_root_trampoline =
        version < Version::new(11, 0, 0) && netloc.user != "root";

    if needs_root_trampoline {
        // Older firmware requires the first user to be "root".
        // Create root, authenticate, then create the target user.
        info!("Adding root user (trampoline for firmware < 11)...");
        AddUserRequest::new(
            "root",
            &netloc.pass,
            pwdgrp::Group::Root,
            pwdgrp::Role::AdminOperatorViewerPtz,
        )
        .send(&client)
        .await?;

        let client = netloc.connect_as("root", &netloc.pass).await?;

        info!("Adding user '{}'...", netloc.user);
        AddUserRequest::new(
            &netloc.user,
            &netloc.pass,
            pwdgrp::Group::Users,
            pwdgrp::Role::AdminOperatorViewerPtz,
        )
        .send(&client)
        .await?;
    } else {
        let group = if netloc.user == "root" {
            pwdgrp::Group::Root
        } else {
            pwdgrp::Group::Users
        };

        info!("Adding user '{}'...", netloc.user);
        AddUserRequest::new(
            &netloc.user,
            &netloc.pass,
            group,
            pwdgrp::Role::AdminOperatorViewerPtz,
        )
        .send(&client)
        .await?;
    }

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

    apply_setup_profile(&client, &version).await?;

    info!("Device initialized");
    Ok(())
}

use anyhow::{bail, Context};
use log::{debug, info, warn};
use rs4a_vapix::{
    apis::{
        applications_config,
        basic_device_info_1::GetAllUnrestrictedPropertiesRequest,
        network_settings_1::{SetGlobalProxyConfigurationData, SetGlobalProxyConfigurationRequest},
        parameter_management, pwdgrp,
        pwdgrp::AddUserRequest,
        ssh_1,
        system_ready_1::SystemReadyRequest,
    },
    protocol_helpers::http::Error,
    Client,
};
use semver::Version;

use crate::Netloc;

#[derive(Clone, Debug, Default, clap::ValueEnum)]
pub enum Profile {
    #[default]
    Default,
    Vlt,
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Profile::Default => write!(f, "default"),
            Profile::Vlt => write!(f, "vlt"),
        }
    }
}

#[derive(Clone, Debug, clap::Args)]
pub struct InitCommand {
    #[command(flatten)]
    pub netloc: Netloc,
    #[arg(long, default_value_t)]
    pub profile: Profile,
}

impl InitCommand {
    pub async fn exec(self) -> anyhow::Result<String> {
        initialize(&self.netloc, &self.profile).await?;
        Ok(String::new())
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
            .send(client)
            .await?;
    } else {
        debug!("Skipping AllowUnsigned (not applicable for firmware {version})");
    }

    Ok(())
}

async fn create_user(netloc: &Netloc, anonymous_client: &Client) -> anyhow::Result<()> {
    debug!("adding initial user");
    let result = AddUserRequest::new(
        &netloc.user,
        &netloc.pass,
        pwdgrp::Group::Root,
        pwdgrp::Role::AdminOperatorViewerPtz,
    )
    .send(anonymous_client)
    .await;

    match result {
        Ok(()) => return Ok(()),
        Err(Error::Service(e)) if e.message() == "not a valid initial admin user" => {}
        Err(e) => return Err(e).context("Failed to create user"),
    }

    debug!("adding root user");
    AddUserRequest::new(
        "root",
        &netloc.pass,
        pwdgrp::Group::Root,
        pwdgrp::Role::AdminOperatorViewerPtz,
    )
    .send(anonymous_client)
    .await
    .context("create root user failed")?;

    debug!("connecting as root");
    let root_client = netloc.connect_as("root").await?;

    debug!("adding initial user using root as springboard");
    AddUserRequest::new(
        &netloc.user,
        &netloc.pass,
        pwdgrp::Group::Root,
        pwdgrp::Role::AdminOperatorViewerPtz,
    )
    .send(&root_client)
    .await
    .context("create initial user failed")?;

    Ok(())
}

pub async fn initialize(netloc: &Netloc, profile: &Profile) -> anyhow::Result<()> {
    info!("Initializing device...");

    let anonymous_client = netloc.connect_anonymous().await?;

    let data = SystemReadyRequest::new().send(&anonymous_client).await?;
    if data.needsetup {
        create_user(netloc, &anonymous_client).await?;
    } else if matches!(profile, Profile::Vlt) {
        info!("Device already set up, skipping root user creation");
    } else {
        bail!("Expected device to be in setup mode, but needsetup is false");
    }

    let client = netloc.connect().await?;

    let data = GetAllUnrestrictedPropertiesRequest::new()
        .send(&client)
        .await
        .context("Failed to query firmware version")?;
    let version = parse_firmware_version(&data.property_list.version)
        .context("Failed to parse firmware version")?;
    debug!("Detected firmware version: {version}");

    info!("Enabling SSH...");
    parameter_management::UpdateRequest::default()
        .network_ssh_enabled(true)
        .send(&client)
        .await?;

    if version >= Version::new(11, 0, 0) {
        info!("Adding SSH user...");
        ssh_1::AddUserRequest::new("ssh", &netloc.pass)
            .send(&client)
            .await
            .context("Failed to add SSH user")?;
    } else {
        debug!("Skipping SSH user creation (not supported on firmware {version})");
    }

    info!("Removing device from known_hosts...");
    match crate::ssh_keygen::remove_known_host(&netloc.host.to_string()) {
        Ok(()) => {}
        Err(e) => warn!("Failed to remove known_hosts entry: {e:?}"),
    }

    apply_setup_profile(&client, &version).await?;

    if matches!(profile, Profile::Vlt) {
        // FIXME: Skip when not supported by firmware
        info!("Setting global proxy configuration...");
        let SetGlobalProxyConfigurationData { .. } = SetGlobalProxyConfigurationRequest::new()
            .http_proxy("http://localhost:8118")
            .https_proxy("http://localhost:8118")
            .no_proxy("127.0.0.12")
            .send(&client)
            .await?;
    }

    info!("Device initialized");
    Ok(())
}

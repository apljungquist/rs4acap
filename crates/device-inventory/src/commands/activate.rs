use std::{borrow::Cow, collections::HashMap};

use anyhow::{bail, Context};
use log::{debug, warn};
use rs4a_vlt::{
    client::Client,
    requests::{Reason, TimeOption},
    responses::{DeviceStatus, Loan},
};
use tokio::task::JoinSet;

use crate::{
    commands::list::probe_device,
    db,
    db::Database,
    db_vlt, fusion,
    fusion::{
        coerce_firmware_version, convert_architecture, inventory_fingerprint, loan_fingerprint,
        BorrowedDevice, DeviceFilter, DeviceFilterParser,
    },
};

async fn try_active(
    offline: bool,
    filter: &DeviceFilter,
    probe: bool,
) -> anyhow::Result<Option<rs4a_dut::Device>> {
    debug!("Searching active...");
    let Some(maybe) = rs4a_dut::Device::from_anywhere()
        .inspect(|d| debug!("Found {} active devices", d.iter().len()))?
        .filter(|d| filter.matches(fusion::BorrowedDevice::from(d), true))
    else {
        return Ok(None);
    };

    let mut architecture = None;
    let mut firmware = None;
    let mut model = None;
    if probe {
        let (_, unrestricted, restricted) = probe_device(
            offline,
            String::new(),
            maybe.host.clone(),
            maybe.http_port,
            maybe.https_port,
            Some(maybe.username.clone()),
            Some(maybe.password.clone()),
        )
        .await?;

        if let Some(restricted) = restricted {
            architecture = Some(convert_architecture(restricted.architecture)?);
        }
        firmware = Some(coerce_firmware_version(unrestricted.version.as_str())?);
        model = Some(Cow::from(unrestricted.prod_short_name));
    }

    match filter.matches_with_properties(
        BorrowedDevice::from(&maybe),
        architecture,
        firmware.as_ref(),
        model,
        false,
    ) {
        true => Ok(Some(maybe)),
        false => Ok(None),
    }
}

async fn try_devices(
    filter: &DeviceFilter,
    client: &Client,
) -> anyhow::Result<Option<rs4a_vlt::responses::Device>> {
    let all = rs4a_vlt::requests::devices().send(client).await?;

    let mut devices = Vec::with_capacity(all.len());
    for device in all {
        if filter.try_matches_vlt_device(&device, true)? && device.status == DeviceStatus::Connected
        {
            devices.push(device);
        }
    }

    devices.sort_by_key(|d| d.id.as_u16());

    let mut devices = devices.into_iter();
    let Some(device) = devices.next() else {
        return Ok(None);
    };
    if devices.next().is_some() {
        warn!("Found more than one device, using the first one");
    }
    Ok(Some(device))
}

async fn try_inventory(
    offline: bool,
    filter: &DeviceFilter,
    db: &Database,
    probe: bool,
) -> anyhow::Result<Option<(String, db::Device)>> {
    debug!("Searching inventory...");
    let mut maybe = db
        .read_devices()
        .inspect(|d| debug!("Found {} devices in inventory", d.len()))?
        .into_iter()
        .filter(|(a, d)| filter.matches(fusion::BorrowedDevice::from((a, d)), true))
        .map(|(a, d)| (inventory_fingerprint(&d), (a, d, None, None, None)))
        .collect::<HashMap<_, _>>();

    if probe {
        let mut join_set = JoinSet::new();
        for (fingerprint, (_, device, ..)) in maybe.iter() {
            join_set.spawn(crate::commands::list::probe_device(
                offline,
                fingerprint.clone(),
                device.host.clone(),
                device.http_port,
                device.https_port,
                Some(device.username.clone()),
                Some(device.password.dangerous_reveal().to_string()),
            ));
        }
        for r in join_set.join_all().await {
            match r {
                Ok((fingerprint, unrestricted, restricted)) => {
                    let value = maybe
                        .get_mut(&fingerprint)
                        .expect("Fingerprint comes from a device already in devices");
                    if let Some(restricted) = restricted {
                        value.2 = Some(convert_architecture(restricted.architecture)?);
                    }
                    value.3 = Some(coerce_firmware_version(&unrestricted.version)?);
                    value.4 = Some(Cow::from(unrestricted.prod_short_name));
                }
                Err(e) => {
                    warn!("Could not get properties for a device: {e:?}");
                }
            }
        }
    }

    let mut definitely = maybe
        .into_values()
        .filter_map(|(alias, device, architecture, firmware, model)| {
            if filter.matches_with_properties(
                fusion::BorrowedDevice::from((&alias, &device)),
                architecture,
                firmware.as_ref(),
                model,
                false,
            ) {
                Some((alias, device))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    definitely.sort_by_key(|(alias, _)| alias.clone());

    let mut definitely = definitely.into_iter();
    let Some((alias, device)) = definitely.next() else {
        return Ok(None);
    };
    if definitely.next().is_some() {
        warn!("More than one matching device found in the inventory, proceeding with the first");
    }
    Ok(Some((alias, device)))
}

async fn try_loans(
    offline: bool,
    filter: &DeviceFilter,
    client: &Client,
    probe: bool,
) -> anyhow::Result<Option<Loan>> {
    debug!("Searching loans...");
    let mut maybe = rs4a_vlt::requests::loans()
        .send(client)
        .await
        .inspect(|d| debug!("Found {} loans", d.len()))?
        .into_iter()
        .filter(|l| filter.matches(fusion::BorrowedDevice::from(l), true))
        .map(|l| (loan_fingerprint(&l), (l, None, None, None)))
        .collect::<HashMap<_, _>>();

    if probe {
        let mut join_set = JoinSet::new();
        for (fingerprint, (loan, ..)) in maybe.iter() {
            join_set.spawn(crate::commands::list::probe_device(
                offline,
                fingerprint.clone(),
                loan.host(),
                Some(loan.http_port()),
                Some(loan.https_port()),
                Some(loan.username.clone()),
                Some(loan.password.clone()),
            ));
        }
        for r in join_set.join_all().await {
            match r {
                Ok((fingerprint, unrestricted, restricted)) => {
                    let value = maybe
                        .get_mut(&fingerprint)
                        .expect("Fingerprint comes from a device already in devices");
                    if let Some(restricted) = restricted {
                        value.1 = Some(convert_architecture(restricted.architecture)?);
                    }
                    value.2 = Some(coerce_firmware_version(&unrestricted.version)?);
                    value.3 = Some(Cow::from(unrestricted.prod_short_name));
                }
                Err(e) => {
                    warn!("Could not get properties for a device: {e:?}");
                }
            }
        }
    }

    let mut definitely = maybe
        .into_values()
        .filter_map(|(loan, architecture, firmware, model)| {
            if filter.matches_with_properties(
                fusion::BorrowedDevice::from(&loan),
                architecture,
                firmware.as_ref(),
                model,
                false,
            ) {
                Some(loan)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    definitely.sort_by_key(|l| l.id);

    let mut definitely = definitely.into_iter();
    let Some(loan) = definitely.next() else {
        return Ok(None);
    };
    if definitely.next().is_some() {
        warn!("Found more than one device loaned, using the first one");
    }
    Ok(Some(loan))
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, clap::ValueEnum)]
pub(crate) enum Destination {
    /// Write information to the filesystem.
    Filesystem,
    /// Print information as a shell script that can be sourced.
    Environment,
}

impl Destination {
    fn activate(&self, device: db::Device) -> anyhow::Result<()> {
        match self {
            Destination::Filesystem => {
                rs4a_dut::Device::from(device).to_fs()?;
            }
            Destination::Environment => {
                // TODO: Consider `unset`ing variables that are not set.
                let envs = crate::env::envs(&device);
                for (key, value) in envs {
                    if let Some(value) = value {
                        println!("export {key}={value}");
                    } else {
                        println!("unset {key}");
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, clap::Parser)]
pub struct ActivateCommand {
    /// Probe devices for properties not indexed by the source.
    ///
    /// Note that devices found only in the VLT source are not probed.
    #[arg(long)]
    probe: bool,
    /// Reason why the device is being borrowed.
    ///
    /// Needed only to create a new VLT loan.
    #[arg(long)]
    reason: Option<String>,
    /// Number of hours to borrow a device.
    ///
    /// Used only when a new VLT loan is created.
    #[arg(long, default_value = "8")]
    hours: u8,
    // Where to store the information about which device is active.
    #[arg(long, default_value = "filesystem")]
    pub(crate) destination: Destination,
    #[command(flatten)]
    device_filter: DeviceFilterParser,
}

impl ActivateCommand {
    pub async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<()> {
        let Self {
            probe,
            reason: new_loan_reason,
            hours: new_loan_duration_hours,
            device_filter,
            destination,
        } = self;
        let device_filter = device_filter.into_filter()?;

        if try_active(offline, &device_filter, probe).await?.is_some() {
            debug!("Found matching active device");
            return Ok(());
        }

        if let Some((alias, device)) = try_inventory(offline, &device_filter, db, probe).await? {
            debug!("Found matching imported device {alias}");
            destination.activate(device)?;
            debug!("Device activated");
            return Ok(());
        }

        let client = db_vlt::client(db, offline)
            .await?
            .context("No active VLT session")?;

        if let Some(loan) = try_loans(offline, &device_filter, &client, probe).await? {
            debug!("Found matching loaned device");
            let (alias, device) = db_vlt::store_parsed(db, vec![loan])?
                .into_iter()
                .next()
                .expect("one loan was passed in so one device is returned");
            debug!("Device stored as {alias}");
            destination.activate(device)?;
            debug!("Device activated");
            return Ok(());
        }

        if let Some(device) = try_devices(&device_filter, &client).await? {
            debug!("Found matching other device: {device:?}");
            let Some(reason) = new_loan_reason else {
                bail!("Cannot create a new loan without specifying a reason");
            };
            let reason = match reason {
                s if Reason::ACAPTest.to_string() == s => Reason::ACAPTest,
                s if Reason::AXISOSTest.to_string() == s => Reason::AXISOSTest,
                s if Reason::FeatureTestDevice.to_string() == s => Reason::FeatureTestDevice,
                s => Reason::Other(s.into()),
            };
            let loan_id = rs4a_vlt::requests::create_loan(
                device.id,
                reason,
                TimeOption::hours_from_now(new_loan_duration_hours),
                device.firmware_version,
            )
            .send(&client)
            .await?
            .id;
            debug!("Created loan {loan_id} in the VLT");
            let loan = rs4a_vlt::requests::loans()
                .send(&client)
                .await?
                .into_iter()
                .find(|l| l.id == loan_id)
                .context("Loans do not include new loan id")?;
            let (alias, device) = db_vlt::store_parsed(db, vec![loan])?
                .into_iter()
                .next()
                .expect("one loan was passed in so one device is returned");
            debug!("Device stored as {alias}");
            destination.activate(device)?;
            debug!("Device activated");
            return Ok(());
        }

        bail!("No matching device found");
    }
}

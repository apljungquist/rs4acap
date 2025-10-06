use anyhow::{bail, Context};
use device_inventory::{db, db::Database, db_vlt, db_vlt::try_device_from_loan};
use log::{debug, warn};
use rs4a_vlt::{
    client::Client,
    requests,
    requests::{Reason, TimeOption},
    responses::{DeviceStatus, Loan},
};

use crate::commands::{
    activate::{ActivateCommand, Destination},
    import::{ImportCommand, Source},
    search,
    search::{SearchCommand, SearchFilter},
};

#[derive(Clone, Debug, clap::Parser)]
pub struct EnsureCommand {
    #[command(flatten)]
    search: SearchCommand,
    /// How to activate the device.
    #[arg(long, default_value = "filesystem")]
    destination: Destination,
}

fn try_active(filter: &SearchFilter) -> anyhow::Result<Option<rs4a_dut::Device>> {
    debug!("Searching active...");
    let device = rs4a_dut::Device::from_anywhere()
        .inspect(|d| debug!("Found {} active devices", d.iter().len()))?
        .filter(|d| filter.matches(search::Device::from(d)));
    Ok(device)
}

async fn try_devices(
    filter: &SearchFilter,
    client: &Client,
) -> anyhow::Result<Option<rs4a_vlt::responses::Device>> {
    let mut other = requests::devices()
        .send(client)
        .await
        .inspect(|d| debug!("Found {} other devices", d.len()))?
        .into_iter()
        .filter(|d| filter.matches(search::Device::from(d)) && d.status == DeviceStatus::Connected)
        .collect::<Vec<_>>();
    other.sort_by_key(|d| d.id.as_u16());
    let mut other = other.into_iter();
    let Some(device) = other.next() else {
        return Ok(None);
    };
    if other.next().is_some() {
        warn!("Found more than one device, using the first one");
    }
    Ok(Some(device))
}

async fn try_inventory(
    filter: &SearchFilter,
    db: &Database,
) -> anyhow::Result<Option<(String, db::Device)>> {
    debug!("Searching inventory...");
    let mut from_inventory = db
        .read_devices()
        .inspect(|d| debug!("Found {} devices in inventory", d.len()))?
        .into_iter()
        .filter(|kv| filter.matches(search::Device::from(kv)))
        .collect::<Vec<_>>();
    from_inventory.sort_by_key(|(alias, _)| alias.clone());
    let mut from_inventory = from_inventory.into_iter();
    let Some((alias, device)) = from_inventory.next() else {
        return Ok(None);
    };

    if from_inventory.next().is_some() {
        warn!("More than matching device found in the inventory, proceeding with the first");
    }
    Ok(Some((alias, device)))
}

async fn try_loans(filter: &SearchFilter, client: &Client) -> anyhow::Result<Option<Loan>> {
    debug!("Searching loans...");
    let mut loaned = requests::loans()
        .send(client)
        .await
        .inspect(|d| debug!("Found {} loans", d.len()))?
        .into_iter()
        .filter(|l| filter.matches(search::Device::from(l)));
    let Some(loan) = loaned.next() else {
        return Ok(None);
    };

    if loaned.next().is_some() {
        warn!("Found more than one device loaned, using the first one");
    }
    Ok(Some(loan))
}

impl EnsureCommand {
    pub async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<()> {
        let Self {
            search,
            destination,
        } = self;
        let search = search.into_filter()?;

        if try_active(&search)?.is_some() {
            debug!("Found matching active device");
            return Ok(());
        }

        if let Some((alias, _)) = try_inventory(&search, db).await? {
            debug!("Found matching imported device");

            ActivateCommand {
                alias: Some(alias),
                destination,
            }
            .exec(db)
            .await?;

            return Ok(());
        }

        let client = db_vlt::client(db, offline)
            .await?
            .context("no active VLT session")?;

        if let Some(loan) = try_loans(&search, &client).await? {
            debug!("Found matching loaned device");
            let (alias, _) = try_device_from_loan(loan)?;

            ImportCommand {
                source: Source::Pool,
            }
            .exec(db, offline)
            .await?;

            ActivateCommand {
                alias: Some(alias.clone()),
                destination,
            }
            .exec(db)
            .await?;

            return Ok(());
        }

        if let Some(device) = try_devices(&search, &client).await? {
            debug!("Found matching other device: {device:?}");
            let loan = requests::create_loan(
                device.id,
                Reason::ACAPTest,
                TimeOption::hours_from_now(8),
                device.firmware_version,
            )
            .send(&client)
            .await?;
            let alias = format!("vlt-{}", loan.loanable.id);

            ImportCommand {
                source: Source::Pool,
            }
            .exec(db, offline)
            .await?;

            ActivateCommand {
                alias: Some(alias.clone()),
                destination,
            }
            .exec(db)
            .await?;

            return Ok(());
        }

        bail!("No matching device found")
    }
}

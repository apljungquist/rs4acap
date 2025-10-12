use std::collections::HashMap;

use anyhow::Context;
use log::warn;

use crate::{
    db::Database,
    db_vlt,
    fusion::{
        active_fingerprint, inventory_fingerprint, loan_fingerprint, Device, DeviceFilterParser,
    },
};

#[derive(Clone, Debug, clap::Parser)]
pub struct ReturnCommand {
    #[command(flatten)]
    device_filter: DeviceFilterParser,
}

impl ReturnCommand {
    pub async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<()> {
        let device_filter = self.device_filter.into_filter()?;

        let client = db_vlt::client(db, offline)
            .await?
            .context("No VLT session")?;

        let mut candidates = rs4a_vlt::requests::loans()
            .send(&client)
            .await?
            .into_iter()
            .map(|loan| (loan_fingerprint(&loan), Device::from_vlt_loan(loan)))
            .collect::<HashMap<_, _>>();

        if candidates.is_empty() {
            warn!("No devices were returned");
            return Ok(());
        }

        if let Some(d) = rs4a_dut::Device::from_anywhere()? {
            if let Some(c) = candidates.get_mut(&active_fingerprint(&d)) {
                c.replace_dut_device(d);
            }
        }

        for (a, d) in db.read_devices()? {
            if let Some(c) = candidates.get_mut(&inventory_fingerprint(&d)) {
                c.replace_inventory_device(a, d);
            }
        }

        let mut candidates = candidates
            .into_values()
            .filter(|d| d.is_matched_by(&device_filter))
            .collect::<Vec<_>>();
        candidates.sort_by(Device::cmp);
        let mut candidates = candidates.into_iter();

        if let Some(candidate) = candidates.next() {
            if candidates.next().is_some() {
                warn!("Found more than one matching loan, only the first will be cancelled");
            }
            let loan_id = candidate
                .loan()
                .as_ref()
                .expect("All candidates are created from a loan")
                .id;
            rs4a_vlt::requests::cancel_loan(loan_id)
                .send(&client)
                .await?;

            let fingerprint = candidate.fingerprint();

            if let Some(d) = rs4a_dut::Device::from_env()? {
                if active_fingerprint(&d) == fingerprint {
                    for key in rs4a_dut::Device::clear_env() {
                        println!("unset {key}")
                    }
                }
            }

            if let Some(d) = rs4a_dut::Device::from_fs()? {
                if active_fingerprint(&d) == fingerprint {
                    rs4a_dut::Device::clear_fs()?;
                }
            }

            let mut from_inventory = db.read_devices()?;
            from_inventory.retain(|_, d| inventory_fingerprint(d) != fingerprint);
            db.write_devices(&from_inventory)?;
        } else {
            warn!("Found no matching loan");
        }

        Ok(())
    }
}

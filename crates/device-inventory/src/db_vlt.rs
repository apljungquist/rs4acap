//! Utilities for connecting the local database to the VLT

use std::collections::HashMap;

use anyhow::bail;
use log::warn;

use crate::{
    db::{Database, Device},
    vlt,
};

/// Add any new devices from the VLT to the local inventory
///
/// # Panics
///
/// This function will panic if offline is true.
pub async fn import(db: &Database, offline: bool) -> anyhow::Result<HashMap<String, Device>> {
    if offline {
        panic!("Cannot fetch devices from a pool when offline");
    }
    let Some(cookie) = db.read_cookie()? else {
        bail!("No login session, please run the login command")
    };
    let loans = vlt::fetch(&cookie).await?;
    store(db, &loans)
}

pub fn store(db: &Database, loans: &str) -> anyhow::Result<HashMap<String, Device>> {
    let loans = vlt::parse(loans)?;
    let mut devices = db.read_devices()?;
    for loan in loans.into_iter() {
        match loan.try_into_device() {
            Ok((alias, device)) => {
                devices.insert(alias, device);
            }
            Err(e) => warn!("Failed to infer IP because {e:?}, skipping this device"),
        }
    }
    // TODO: Remove expired devices
    db.write_devices(&devices)?;
    Ok(devices)
}

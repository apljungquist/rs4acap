//! Utilities for connecting the local database to the VLT

use std::collections::{HashMap, HashSet};

use anyhow::Context;
use rs4a_vlt::{
    client::Client,
    responses::{Loan, Loanable},
};

use crate::{
    db::{Database, Device},
    psst::Password,
};

/// Prepare a client builder
///
/// # Panics
///
/// This function will panic if offline is true.
pub async fn client(db: &Database, offline: bool) -> anyhow::Result<Option<Client>> {
    if offline {
        panic!("Cannot fetch devices from a pool when offline");
    }
    let Some(cookie) = db.read_cookie()? else {
        return Ok(None);
    };

    Some(Client::try_new(cookie)).transpose()
}

/// Sync devices from the VLT to the local inventory
///
/// Adds new devices from active loans and removes stale `vlt-*` devices
/// that no longer correspond to active loans.
///
/// # Panics
///
/// This function will panic if offline is true.
pub async fn sync(db: &Database, offline: bool) -> anyhow::Result<HashMap<String, Device>> {
    let client = client(db, offline)
        .await?
        .context("No login session, please run the login command")?;
    let loans = rs4a_vlt::requests::loans().send(&client).await?;
    sync_parsed(db, loans)
}

fn sync_parsed(db: &Database, loans: Vec<Loan>) -> anyhow::Result<HashMap<String, Device>> {
    let mut devices = db.read_devices()?;
    let mut vlt_aliases: HashSet<String> = HashSet::new();
    for loan in loans.into_iter() {
        let (alias, device) = device_from_loan(loan);
        vlt_aliases.insert(alias.clone());
        devices.insert(alias, device);
    }
    devices.retain(|alias, _| !alias.starts_with("vlt-") || vlt_aliases.contains(alias));
    db.write_devices(&devices)?;
    Ok(devices)
}

pub fn device_from_loan(loan: Loan) -> (String, Device) {
    let host = loan.host();
    let http_port = loan.http_port();
    let https_port = loan.https_port();
    let ssh_port = loan.ssh_port();
    let Loan {
        username,
        password,
        loanable:
            Loanable {
                external_ip: _,
                internal_ip: _,
                id,
                model,
                ..
            },
        ..
    } = loan;
    (
        format!("vlt-{id}"),
        Device {
            model: Some(model),
            host,
            username,
            password: Password::new(password),
            http_port: Some(http_port),
            https_port: Some(https_port),
            ssh_port: Some(ssh_port),
            https_self_signed: true,
        },
    )
}

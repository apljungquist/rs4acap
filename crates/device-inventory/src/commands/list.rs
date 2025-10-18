use std::collections::{hash_map::Entry, HashMap};

use anyhow::Context;
use log::warn;
use rs4a_vapix::{
    apis,
    basic_device_info_1::{AllProperties, RestrictedProperties, UnrestrictedProperties},
    json_rpc_http::JsonRpcHttp,
};
use rs4a_vlt::requests;
use tokio::task::JoinSet;
use url::Host;

use crate::{
    db::Database,
    db_vlt,
    fusion::{
        active_fingerprint, inventory_fingerprint, loan_fingerprint, mdns_fingerprint,
        other_fingerprint, Device, DeviceFilterParser,
    },
    mdns_source,
};

struct Table {
    aliases: Vec<String>,
    architectures: Vec<String>,
    firmware: Vec<String>,
    hosts: Vec<String>,
    models: Vec<String>,
    serials: Vec<String>,
    statuses: Vec<String>,
    priorities: Vec<String>,
}

impl Table {
    fn new() -> Self {
        Self {
            aliases: vec!["ALIAS".to_string()],
            architectures: vec!["ARCHITECTURE".to_string()],
            firmware: vec!["FIRMWARE".to_string()],
            hosts: vec!["HOST".to_string()],
            models: vec!["MODEL".to_string()],
            serials: vec!["SERIAL".to_string()],
            statuses: vec!["STATUS".to_string()],
            priorities: vec!["PRIORITIES".to_string()],
        }
    }

    fn push(&mut self, row: Device) {
        self.aliases
            .push(row.alias().unwrap_or_default().to_string());
        self.architectures.push(
            row.architecture()
                .map(|a| a.as_str().to_string())
                .unwrap_or_default(),
        );
        self.firmware
            .push(row.firmware().map(|v| v.to_string()).unwrap_or_default());
        self.hosts.push(row.host().to_string());
        self.models
            .push(row.model().unwrap_or_default().to_string());
        self.serials
            .push(row.serial().map(|s| s.to_string()).unwrap_or_default());
        self.statuses.push(
            row.status()
                .map(|s| s.as_str().to_string())
                .unwrap_or_default(),
        );
        self.priorities.push(
            row.priorities()
                .into_iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
    }

    pub fn pretty_print(self) {
        let Self {
            aliases,
            architectures,
            firmware,
            hosts,
            models,
            serials,
            statuses,
            priorities,
        } = self;

        let alias_width = 1 + aliases.iter().map(|s| s.len()).max().unwrap();
        let firmware_width = 1 + firmware.iter().map(|s| s.len()).max().unwrap();
        let architecture_width = 1 + architectures.iter().map(|s| s.len()).max().unwrap();
        let model_width = 1 + models.iter().map(|s| s.len()).max().unwrap();
        let serial_width = 1 + serials.iter().map(|s| s.len()).max().unwrap();
        let status_width = 1 + statuses.iter().map(|s| s.len()).max().unwrap();
        let priority_width = 1 + priorities.iter().map(|s| s.len()).max().unwrap();

        for (((((((priority, alias), model), host), architecture), serial), status), firmware) in
            priorities
                .into_iter()
                .zip(aliases.into_iter())
                .zip(models.into_iter())
                .zip(hosts.into_iter())
                .zip(architectures.into_iter())
                .zip(serials.into_iter())
                .zip(statuses.into_iter())
                .zip(firmware.into_iter())
        {
            println!(
                "{priority:priority_width$} {status:status_width$} {alias:alias_width$} {serial:serial_width$} {model:model_width$} {architecture:architecture_width$} {firmware:firmware_width$} {host}"
            );
        }
    }
}

async fn probe_device(
    offline: bool,
    fingerprint: String,
    host: Host,
    http_port: Option<u16>,
    https_port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
) -> anyhow::Result<(String, UnrestrictedProperties, Option<RestrictedProperties>)> {
    assert!(!offline);
    match (username, password) {
        (Some(username), Some(password)) => {
            let client = rs4a_vapix::Client::builder(host)
                .basic_authentication(&username, &password)
                .plain_port(http_port)
                .secure_port(https_port)
                .with_inner(|b| b.danger_accept_invalid_certs(true))
                .build_with_automatic_scheme()
                .await
                .context("Could not create client")?;

            let AllProperties {
                unrestricted,
                restricted,
            } = apis::basic_device_info_1::get_all_properties()
                .send(&client)
                .await?
                .property_list;
            Ok((fingerprint, unrestricted, Some(restricted)))
        }
        _ => {
            let client = rs4a_vapix::Client::builder(host)
                .plain_port(http_port)
                .secure_port(https_port)
                .with_inner(|b| b.danger_accept_invalid_certs(true))
                .build_with_automatic_scheme()
                .await
                .context("Could not create client")?;

            let unrestricted = apis::basic_device_info_1::get_all_unrestricted_properties()
                .send(&client)
                .await?
                .property_list;
            Ok((fingerprint, unrestricted, None))
        }
    }
}

#[derive(Clone, Debug, clap::Parser)]
pub struct ListCommand {
    /// Probe devices for properties not indexed by the source.
    ///
    /// Note that devices found only in the VLT source are not probed.
    #[arg(long)]
    probe: bool,
    /// Consider devices from mDNS.
    #[arg(long, env = "DI_MDNS")]
    mdns: bool,
    /// Consider loans and devices from the VLT.
    #[arg(long, env = "DI_VLT")]
    vlt: bool,
    #[command(flatten)]
    device_filter: DeviceFilterParser,
}

impl ListCommand {
    pub async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<()> {
        let Self {
            probe,
            mdns,
            vlt,
            device_filter,
        } = self;
        let device_filter = device_filter.into_filter()?;

        let mut devices: HashMap<String, Device> = HashMap::new();

        if let Some(d) = rs4a_dut::Device::from_anywhere()? {
            let f = active_fingerprint(&d);
            match devices.entry(f) {
                Entry::Occupied(mut e) => {
                    e.get_mut().replace_dut_device(d);
                }
                Entry::Vacant(e) => {
                    e.insert(Device::from_dut_device(d));
                }
            };
        }

        for (a, d) in db.read_devices()? {
            let f = inventory_fingerprint(&d);
            match devices.entry(f) {
                Entry::Occupied(mut e) => {
                    e.get_mut().replace_inventory_device(a, d);
                }
                Entry::Vacant(e) => {
                    e.insert(Device::from_inventory_device(a, d));
                }
            };
        }

        if mdns {
            for d in mdns_source::discover_devices().await? {
                let f = mdns_fingerprint(&d);
                match devices.entry(f) {
                    Entry::Occupied(mut e) => {
                        e.get_mut().replace_mdns_device(d);
                    }
                    Entry::Vacant(e) => {
                        e.insert(Device::from_mdns_device(d));
                    }
                };
            }
        }

        let vlt_client = match vlt {
            true => Some(
                db_vlt::client(db, offline)
                    .await?
                    .context("VLT is not configured, skipping VLT devices")?,
            ),
            false => None,
        };

        if let Some(client) = vlt_client.as_ref() {
            for d in requests::loans().send(client).await? {
                let f = loan_fingerprint(&d);
                match devices.entry(f) {
                    Entry::Occupied(mut e) => e.get_mut().replace_vlt_loan(d),
                    Entry::Vacant(e) => {
                        e.insert(Device::from_vlt_loan(d));
                        None
                    }
                };
            }
        }

        if probe {
            let mut join_set = JoinSet::new();
            for device in devices.values() {
                join_set.spawn(probe_device(
                    offline,
                    device.fingerprint(),
                    device.host(),
                    device
                        .http_port()
                        .expect("all devices added have a http port"),
                    device
                        .https_port()
                        .expect("all devices added have a https port"),
                    device.username(),
                    device.password(),
                ));
            }
            for r in join_set.join_all().await {
                match r {
                    Ok((fingerprint, unrestricted, restricted)) => {
                        let device = devices
                            .get_mut(&fingerprint)
                            .expect("Fingerprint comes from a device already in devices");
                        device.add_unrestricted_properties(unrestricted)?;
                        if let Some(restricted) = restricted {
                            device.add_restricted_properties(restricted)?;
                        }
                    }
                    Err(e) => {
                        warn!("Could not get properties for a device: {e:?}");
                    }
                }
            }
        }

        if let Some(client) = vlt_client.as_ref() {
            for d in requests::devices().send(client).await? {
                let f = other_fingerprint(&d);
                match devices.entry(f) {
                    Entry::Occupied(mut e) => e.get_mut().add_vlt_device(d)?,
                    Entry::Vacant(e) => {
                        e.insert(Device::from_vlt_device(d)?);
                    }
                };
            }
        }

        let mut devices = devices.into_values().collect::<Vec<_>>();
        devices.sort_by(Device::cmp);

        let mut table = Table::new();
        for device in devices {
            if device.is_matched_by(&device_filter) {
                table.push(device);
            }
        }
        table.pretty_print();

        Ok(())
    }
}

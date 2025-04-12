use std::{collections::HashMap, iter::once, net::ToSocketAddrs, time::Duration};

use itertools::Itertools;
use log::{debug, error};
use zeroconf::{
    prelude::{TEventLoop, TMdnsBrowser, TTxtRecord},
    MdnsBrowser, ServiceDiscovery, ServiceType,
};

// TODO: Consider gathering information from more services
// The ones axis use are documented on https://help.axis.com/en-us/axis-os-knowledge-base#bonjour
// and I have a vague memory of reading that avahi supports browsing services with any name.
fn discover_services() -> anyhow::Result<Vec<ServiceDiscovery>> {
    let mut result = Vec::new();

    for name in ["axis-video", "axis-bwsc", "axis-nvr"] {
        let service_type =
            ServiceType::new(name, "tcp").expect("hard coded arguments are known to be valid");
        let mut browser = MdnsBrowser::new(service_type);

        let (tx, rx) = std::sync::mpsc::channel();
        browser.set_service_discovered_callback(Box::new(move |r, _| {
            debug!("Discovered {r:?}");
            match tx.send(r) {
                Ok(()) => (),
                Err(e) => {
                    error!("Could not send value because {e:?}");
                }
            }
        }));

        // TODO: Consider changing the waiting strategy
        browser.browse_services()?.poll(Duration::from_secs(1))?;
        drop(browser);

        while let Ok(s) = rx.recv_timeout(Duration::from_secs(1)) {
            result.push(s?);
        }
    }
    Ok(result)
}

fn flat(service: &ServiceDiscovery) -> anyhow::Result<HashMap<String, String>> {
    let mut flat = HashMap::new();
    flat.insert("Name".to_string(), service.name().to_string());

    let host = service.host_name();
    let port = service.port();
    let address = service.address();
    flat.insert("IP".to_string(), address.to_string());
    let addr = format!("{host}:{port}");
    debug!("Resolving address: {}", &addr);
    for (i, ip) in &addr
        .to_socket_addrs()?
        .filter(|a| &a.ip().to_string() != address)
        .enumerate()
    {
        flat.insert(format!("IP ({})", i + 2), ip.ip().to_string());
    }

    if let Some(txt) = service.txt() {
        if let Some(mac_address) = txt.get("macaddress") {
            flat.insert("MAC".to_string(), mac_address);
        }
    }

    Ok(flat)
}

fn print_table(row: &[HashMap<String, String>]) {
    let columns = row
        .iter()
        .flat_map(|d| d.keys())
        .unique()
        .sorted()
        .collect_vec();

    let column_widths = columns
        .iter()
        .map(|c| {
            row.iter()
                .map(|d| d[*c].len())
                .chain(once(c.len()))
                .max()
                .unwrap()
                + 2
        })
        .collect_vec();

    for (c, w) in columns.iter().zip(column_widths.iter()) {
        print!("{c:w$}");
    }
    println!();

    for row in row {
        for (c, w) in columns.iter().zip(column_widths.iter()) {
            print!("{:w$}", row.get(*c).map(|s| s.as_str()).unwrap_or_default());
        }
        println!();
    }
}

#[derive(clap::Parser, Debug, Clone)]
pub struct DiscoverDevicesCommand {}

impl DiscoverDevicesCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let found = discover_services()?;
        let mut flattened = Vec::new();
        for s in found {
            match flat(&s) {
                Ok(f) => flattened.push(f),
                Err(e) => {
                    error!("Could not flatten service {s:?} because {e:?}")
                }
            }
        }
        print_table(&flattened);
        Ok(())
    }
}

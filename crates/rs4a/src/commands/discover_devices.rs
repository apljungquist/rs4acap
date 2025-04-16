use std::{collections::HashMap, iter::once, net::ToSocketAddrs, time::Duration};

use anyhow::Context;
use itertools::Itertools;
use log::{debug, error, warn};
use rs4a_vapix::{basic_device_info_1::UnrestrictedProperties, system_ready_1::SystemreadyData};
use tokio::task::JoinSet;
use url::Host;
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
    for (i, ip) in addr
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
pub struct DiscoverDevicesCommand {
    /// Probe devices for additional information
    #[arg(long)]
    probe: bool,
}

async fn probe(host: String, addr: String) -> anyhow::Result<(String, HashMap<String, String>)> {
    let mut details = HashMap::new();
    let client = rs4a_vapix::Client::detect_scheme(
        &Host::parse(&addr)?,
        reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?,
    )
    .await
    .context("Could not create client")?;

    let SystemreadyData {
        needsetup,
        systemready,
        ..
    } = client.system_ready_1().system_ready().send().await?;
    details
        .insert("Need Setup".to_string(), needsetup.to_string())
        .inspect(|_| panic!("Each key is created at most once"));
    details
        .insert("System Ready".to_string(), systemready.to_string())
        .inspect(|_| panic!("Each key is created at most once"));

    let UnrestrictedProperties {
        build_date,
        hardware_id,
        prod_nbr,
        prod_short_name,
        prod_type,
        serial_number,
        version,
        ..
    } = client
        .basic_device_info_1()
        .get_all_unrestricted_properties()
        .send()
        .await?
        .property_list;
    details
        .insert("Build Date".to_string(), build_date)
        .inspect(|_| panic!("Each key is created at most once"));
    details
        .insert("Hardware ID".to_string(), hardware_id)
        .inspect(|_| panic!("Each key is created at most once"));
    details
        .insert("Product Short Name".to_string(), prod_short_name)
        .inspect(|_| panic!("Each key is created at most once"));
    details
        .insert("Product Type".to_string(), prod_type)
        .inspect(|_| panic!("Each key is created at most once"));
    details
        .insert("Product Number".to_string(), prod_nbr)
        .inspect(|_| panic!("Each key is created at most once"));
    details
        .insert("Serial Number".to_string(), serial_number)
        .inspect(|_| panic!("Each key is created at most once"));
    details
        .insert("Version".to_string(), version)
        .inspect(|_| panic!("Each key is created at most once"));

    Ok((host, details))
}

impl DiscoverDevicesCommand {
    pub async fn exec(self) -> anyhow::Result<()> {
        let found = discover_services()?;
        let mut flattened = HashMap::new();
        for s in &found {
            match flat(s) {
                Ok(f) => {
                    flattened
                        .insert(s.host_name().clone(), f)
                        .inspect(|s| warn!("Overwriting service {s:?}"));
                }
                Err(e) => {
                    error!("Could not flatten service {s:?} because {e:?}")
                }
            }
        }
        if self.probe {
            let mut join_set = JoinSet::new();
            for s in &found {
                join_set.spawn(probe(s.host_name().clone(), s.address().clone()));
            }
            while let Some(r) = join_set.join_next().await {
                let (host_name, new_info) = r??;
                let all_info = flattened
                    .get_mut(&host_name)
                    .expect("The keys augmented are a subset of the keys previously inserted");
                for (k, v) in new_info {
                    all_info
                        .insert(k, v)
                        .inspect(|_| panic!("Each key is created at most once"));
                }
            }
        }
        print_table(&flattened.into_values().collect_vec());
        Ok(())
    }
}

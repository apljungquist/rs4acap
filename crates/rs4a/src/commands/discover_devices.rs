use std::{collections::HashMap, iter::once, time::Duration};

use anyhow::Context;
use futures_util::{pin_mut, stream::StreamExt};
use itertools::Itertools;
use log::{debug, error, info, warn};
use mdns::{RecordKind, Response};
use rs4a_vapix::{basic_device_info_1::UnrestrictedProperties, system_ready_1::SystemreadyData};
use tokio::{
    task::JoinSet,
    time::{error::Elapsed, timeout},
};
use url::Host;

// TODO: Consider gathering information from more services
// The ones axis use are documented on https://help.axis.com/en-us/axis-os-knowledge-base#bonjour
// and I have a vague memory of reading that avahi supports browsing services with any name.
async fn discover_services() -> anyhow::Result<Vec<Response>> {
    let mut result = HashMap::new();

    for name in ["axis-video", "axis-bwsc", "axis-nvr"] {
        let service_name = format!("_{name}._tcp.local");
        info!("Discovering services with name {service_name}");
        let stream = mdns::discover::all(&service_name, Duration::from_secs(1))?.listen();
        pin_mut!(stream);

        loop {
            let response = match timeout(Duration::from_secs(2), stream.next()).await {
                Err(Elapsed { .. }) => {
                    debug!("Discovery timeout elapsed");
                    break;
                }
                Ok(None) => unreachable!("The underlying stream selects on an infinite interval"),
                Ok(Some(r)) => r,
            };
            let response = response?;
            let Some(hostname) = response.hostname().map(String::from) else {
                warn!("Got a response without a hostname: {response:?}");
                continue;
            };
            // I think this should happen only once we have sent the second request.
            // By then all responses from the first request have hopefully been received.
            if result.insert(hostname, response).is_some() {
                break;
            }
        }
    }
    Ok(result.into_values().collect())
}

fn flat(service: &Response) -> anyhow::Result<HashMap<String, String>> {
    let mut flat = HashMap::new();

    // I don't know why, but `Record::hostname()` is not what I consider a hostname.
    if let Some(name) = service.hostname() {
        flat.insert("Name".to_string(), name.to_string());
    }

    if let Some(hostname) = service.records().find_map(|r| match &r.kind {
        RecordKind::SRV { target, .. } => Some(target.clone()),
        _ => None,
    }) {
        flat.insert("Host".to_string(), hostname.to_string());
    }

    let address = service.ip_addr().map(|a| a.to_string());
    if let Some(address) = &address {
        flat.insert("IP".to_string(), address.clone());
    }

    for (i, ip) in service
        .records()
        .filter_map(|r| match r.kind {
            RecordKind::A(a) => Some(a.to_string()),
            RecordKind::AAAA(a) => Some(a.to_string()),
            _ => None,
        })
        .filter(|a| Some(a) != address.as_ref())
        .enumerate()
    {
        flat.insert(format!("IP ({})", i + 2), ip)
            .inspect(|_| panic!("Each key is created at most once"));
    }

    for txt in service.txt_records() {
        if let Some(mac_address) = txt.strip_prefix("macaddress=") {
            flat.insert("MAC".to_string(), mac_address.to_string())
                .inspect(|old| warn!("Overwriting MAC {old:?}"));
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
                .filter_map(|d| d.get(*c).map(String::len))
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
        let found = discover_services().await?;
        let mut flattened = HashMap::new();
        for s in &found {
            match flat(s) {
                Ok(f) => {
                    flattened
                        .insert(format!("{s:?}"), f)
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
                if let Some(addr) = s.ip_addr() {
                    join_set.spawn(probe(format!("{s:?}"), addr.to_string()));
                } else {
                    warn!("Service {s:?} has no IP address");
                }
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

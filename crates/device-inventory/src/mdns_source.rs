use std::{collections::HashMap, fmt::Write, str::FromStr, time::Duration};

use futures_util::{pin_mut, stream::StreamExt};
use log::{debug, info, warn};
use macaddr::MacAddr6;
use mdns::{RecordKind, Response};
use tokio::{
    task::JoinSet,
    time::{error::Elapsed, timeout},
};
use url::Host;

#[derive(Debug)]
pub struct Device {
    pub host: Host,
    pub mac: MacAddr6,
}

impl Device {
    pub fn to_serial(&self) -> String {
        self.mac.as_bytes().iter().fold(String::new(), |mut f, b| {
            write!(f, "{b:02X}").unwrap();
            f
        })
    }
}

async fn discover_service(name: &str) -> anyhow::Result<HashMap<String, Response>> {
    let service_name = format!("_{name}._tcp.local");
    info!("Discovering services with name {service_name}");
    let stream = mdns::discover::all(&service_name, Duration::from_secs(1))?.listen();
    pin_mut!(stream);

    let mut result = HashMap::new();
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
        // By then, all responses from the first request have hopefully been received.
        if result.insert(hostname, response).is_some() {
            break;
        }
    }
    Ok(result)
}

// TODO: Consider gathering information from more services
// The ones axis use are documented on https://help.axis.com/en-us/axis-os-knowledge-base#bonjour
// and I have a vague memory of reading that avahi supports browsing services with any name.
async fn discover_services() -> anyhow::Result<Vec<Response>> {
    let mut join_set = JoinSet::new();
    for name in ["axis-video", "axis-bwsc", "axis-nvr"] {
        join_set.spawn(discover_service(name));
    }
    let mut responses = HashMap::new();
    for result in join_set.join_all().await {
        for (k, v) in result? {
            responses.insert(k, v);
        }
    }
    Ok(responses.into_values().collect())
}

fn device(service: Response) -> Option<Device> {
    let Some(host) = service.records().find_map(|r| match &r.kind {
        RecordKind::SRV { target, .. } => Host::parse(target)
            .inspect_err(|e| debug!("Could not parse host {e:?}"))
            .ok(),
        _ => None,
    }) else {
        warn!("Response has no SRV record with a valid Host target, skipping");
        return None;
    };

    let Some(mac) = service.txt_records().find_map(|s| {
        s.strip_prefix("macaddress=").and_then(|mac| {
            MacAddr6::from_str(mac)
                .inspect_err(|e| debug!("Could not parse MacAddr6 {e:?}"))
                .ok()
        })
    }) else {
        warn!("Response has no TXT record with a valid MAC address, skipping");
        return None;
    };

    Some(Device { host, mac })
}

pub async fn discover_devices() -> anyhow::Result<Vec<Device>> {
    Ok(discover_services()
        .await?
        .into_iter()
        .flat_map(device)
        .collect())
}

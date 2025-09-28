//! Utilities for working with the VLT

use std::{future::Future, net::Ipv4Addr};

use anyhow::{bail, Context};
use reqwest::{header::COOKIE, Client};
use serde::Deserialize;
use serde_json::value::RawValue;
use url::Host;

use crate::{db::Device, psst::Password};

#[derive(Debug, Deserialize)]
struct Response<'a> {
    success: bool,
    #[serde(borrow)]
    data: &'a RawValue,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Loan {
    username: String,
    password: Password,
    loanable: Loanable,
}

impl Loan {
    fn external_ip(&self) -> anyhow::Result<Ipv4Addr> {
        let addr = &self.loanable.external_ip;
        let addr = Host::parse(addr).context("External IP is not a valid host")?;
        let Host::Ipv4(addr) = addr else {
            bail!("External IP is not an IPv4 address");
        };
        Ok(addr)
    }

    fn base_port(&self) -> anyhow::Result<u16> {
        let (_, port) = self
            .loanable
            .internal_ip
            .split_once(':')
            .context("Internal IP has no port")?;
        let port: u16 = port
            .parse()
            .context("Internal IP port is not a valid port number")?;
        Ok(port)
    }

    fn port_suffix(&self) -> anyhow::Result<u16> {
        let external = self.external_ip()?;
        let [_, _, o2, o3] = external.octets();
        Ok(1_000 * o2 as u16 + o3 as u16)
    }

    fn http_port(&self) -> anyhow::Result<u16> {
        let from_base_port = self.base_port()?;
        let from_suffix = 10_000 + self.port_suffix()?;
        assert_eq!(from_base_port, from_suffix);
        Ok(from_base_port)
    }

    fn https_port(&self) -> anyhow::Result<u16> {
        Ok(40_000 + self.port_suffix()?)
    }

    fn ssh_port(&self) -> anyhow::Result<u16> {
        Ok(20_000 + self.port_suffix()?)
    }

    pub fn try_into_device(self) -> anyhow::Result<(String, Device)> {
        let http_port = self.http_port()?;
        let https_port = self.https_port()?;
        let ssh_port = self.ssh_port()?;
        let Loan {
            username,
            password,
            loanable:
                Loanable {
                    external_ip: _,
                    internal_ip: _,
                    id,
                    model,
                },
        } = self;
        Ok((
            format!("vlt-{}", id),
            Device {
                model: Some(model),
                host: Host::Ipv4(Ipv4Addr::from([195, 60, 68, 14])),
                username,
                password,
                http_port: Some(http_port),
                https_port: Some(https_port),
                ssh_port: Some(ssh_port),
            },
        ))
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Loanable {
    external_ip: String,
    internal_ip: String,
    id: usize,
    model: String,
}

pub fn parse(response: &str) -> anyhow::Result<Vec<Loan>> {
    let Response { success, data } = serde_json::from_str(response)?;
    let data = serde_json::from_str::<Vec<Loan>>(data.get())
        .with_context(|| format!("Success was {success}"))?;
    Ok(data)
}

pub async fn fetch(cookie: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://www.axis.com/partner_pages/adp_virtual_loan_tool/api/user/loans")
        .header(COOKIE, cookie)
        .send()
        .await
        .context("Failed to fetch loans")?;
    let status = response.status();
    let text = response
        .text()
        .await
        .with_context(|| format!("Failed to fetch loans, status was {status}"))?;
    Ok(text)
}

pub trait Request: Send + Sized {
    type ResponseData: for<'de> Deserialize<'de>;

    fn tail(&self) -> &'static str;
    fn send(
        self,
        client: &Client,
    ) -> impl Future<Output = anyhow::Result<Self::ResponseData>> + Send {
        async move {
            let url = format!(
                "https://www.axis.com/partner_pages/adp_virtual_loan_tool/api/user/{}",
                self.tail()
            );
            let response = client
                .get(&url)
                .send()
                .await
                .with_context(|| format!("Send to {url}"))?;
            let status = response.status();
            let text = response
                .text()
                .await
                .with_context(|| format!("Get text from {status} response"))?;
            let Response { success, data } = serde_json::from_str(&text)?;
            let data = serde_json::from_str::<Self::ResponseData>(data.get())
                .with_context(|| format!("Deserialize data from success={success} payload"))?;
            Ok(data)
        }
    }
}

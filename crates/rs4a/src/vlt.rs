//! Utilities for working with the Virtual Loan Tool

use std::net::Ipv4Addr;

use anyhow::{anyhow, bail, Context};
use log::debug;
use serde::Deserialize;
use serde_json::value::RawValue;
use url::Host;

use crate::psst::Password;

#[derive(Debug, Deserialize)]
pub struct Response<'a> {
    success: bool,
    #[serde(borrow)]
    data: &'a RawValue,
}
pub fn parse(response: &str) -> anyhow::Result<Loan> {
    let Response { success, data } = serde_json::from_str(response)?;
    let data = serde_json::from_str::<Vec<Loan>>(data.get())
        .with_context(|| format!("Success was {success}"))?;
    let mut loans = data.into_iter();
    let loan = loans.next().ok_or_else(|| anyhow!("No loans found"))?;
    if loans.next().is_some() {
        bail!("Multiple loans found");
    }
    debug!("Internal ip was {}", loan.loanable.internal_ip);
    Ok(loan)
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
    pub fn effective_ip(&self) -> anyhow::Result<Ipv4Addr> {
        let external = self.external_ip()?;
        let [_, _, _, last] = external.octets();
        Ok(Ipv4Addr::from([195, 60, 68, last]))
    }

    pub fn username(&self) -> &str {
        &self.username
    }
    pub fn password(&self) -> &Password {
        &self.password
    }

    pub fn http_port(&self) -> u16 {
        12051
    }

    pub fn https_port(&self) -> u16 {
        42051
    }

    pub fn ssh_port(&self) -> u16 {
        22051
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Loanable {
    pub internal_ip: String,
    pub external_ip: String,
}

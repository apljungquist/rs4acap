//! Facilities for building and executing requests.
use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    marker::PhantomData,
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    client::Client,
    responses::{parse_data, Device, FirmwareVersion, Loan, LoanId, LoanableId, NewLoan},
};

pub struct GenericRequest<Rq, Rp> {
    method: Method,
    path: String,
    body: Option<Rq>,
    _request: PhantomData<Rq>,
    _response: PhantomData<Rp>,
}

impl<Rq, Rp> GenericRequest<Rq, Rp>
where
    Rq: Serialize + Send,
    Rp: Clone + for<'de> Deserialize<'de> + Serialize + Send + Sync + Debug + 'static,
{
    pub async fn send(self, client: &Client) -> anyhow::Result<Rp> {
        let Self {
            method, path, body, ..
        } = self;
        let url = format!("https://www.axis.com/partner_pages/adp_virtual_loan_tool/api/{path}",);
        let response = if let Some(body) = body {
            client.0.request(method, &url).json(&body).send().await
        } else {
            client.0.request(method, &url).send().await
        }
        .with_context(|| format!("Send to {url}"))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .with_context(|| format!("Get text from {status} response"))?;
        parse_data(&text).with_context(|| format!("{status}"))
    }
}

enum TimeUnit {
    Days,
    Hours,
}
pub struct TimeOption {
    start: DateTime<Utc>,
    count: u8,
    unit: TimeUnit,
}

impl TimeOption {
    pub fn days_from_now(count: u8) -> Self {
        Self {
            start: Utc::now(),
            count,
            unit: TimeUnit::Days,
        }
    }

    pub fn hours_from_now(count: u8) -> Self {
        Self {
            start: Utc::now(),
            count,
            unit: TimeUnit::Hours,
        }
    }

    fn end(&self) -> DateTime<Utc> {
        match self.unit {
            TimeUnit::Days => self.start + chrono::Duration::days(self.count as i64),
            TimeUnit::Hours => self.start + chrono::Duration::hours(self.count as i64),
        }
    }

    fn start(&self) -> DateTime<Utc> {
        self.start
    }

    fn unit(&self) -> String {
        match self.unit {
            TimeUnit::Days => "days".to_string(),
            TimeUnit::Hours => "hours".to_string(),
        }
    }
}

#[non_exhaustive]
pub enum Reason {
    ACAPTest,
    AXISOSTest,
    IntegrationTest,
    FeatureTestDevice,
    Other(Cow<'static, str>),
}

impl Reason {
    pub fn other(s: impl Into<Cow<'static, str>>) -> Self {
        Self::Other(s.into())
    }
}

impl Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ACAPTest => write!(f, "ACAP test"),
            Self::AXISOSTest => write!(f, "AXIS OS Test"),
            Self::IntegrationTest => write!(f, "Integration test"),
            Self::FeatureTestDevice => write!(f, "Feature test device"),
            Self::Other(s) => write!(f, "Other: {s}"),
        }
    }
}

/// Borrows a device
pub fn create_loan(
    id: LoanableId,
    reason: Reason,
    when: TimeOption,
    firmware: FirmwareVersion,
) -> GenericRequest<Value, NewLoan> {
    GenericRequest {
        method: Method::POST,
        path: "user/loans".to_string(),
        body: Some(json!({
            "reason": reason.to_string(),
            "loan_start": when.start(),
            "loan_end":when.end(),
            "loanable_id":id,
            "selected_firmware":firmware,
            "time_option":when.unit()
        })),
        _request: PhantomData,
        _response: PhantomData,
    }
}

/// Return a borrowed device.
pub fn cancel_loan(loan_id: LoanId) -> GenericRequest<(), ()> {
    GenericRequest {
        method: Method::POST,
        path: format!("user/loans/{loan_id}/cancel"),
        body: None,
        _request: PhantomData,
        _response: PhantomData,
    }
}

/// Fetch all ongoing loans for the current user.
pub fn loans() -> GenericRequest<(), Vec<Loan>> {
    GenericRequest {
        method: Method::GET,
        path: "user/loans".to_string(),
        body: None,
        _request: PhantomData,
        _response: PhantomData,
    }
}

/// Fetch all devices listed in VLT.
///
/// Note that this excludes any devices on-loan to the current user.
pub fn devices() -> GenericRequest<(), Vec<Device>> {
    GenericRequest {
        method: Method::GET,
        path: "user/devices".to_string(),
        body: None,
        _request: PhantomData,
        _response: PhantomData,
    }
}

use std::borrow::Cow;

use anyhow::Context;
use device_inventory::{db, db::Database, db_vlt, db_vlt::try_device_from_loan};
use rs4a_vlt::{
    requests,
    responses::{DeviceArchitecture, DeviceStatus, Loan},
};

// TODO: Consider making these mandatory.
pub struct Device<'a> {
    pub(crate) alias: Option<Cow<'a, str>>,
    pub(crate) architecture: Option<DeviceArchitecture>,
    pub(crate) model: Option<&'a str>,
    pub(crate) status: Option<DeviceStatus>,
}

impl<'a> From<&'a (String, db::Device)> for Device<'a> {
    fn from(value: &'a (String, db::Device)) -> Self {
        Self {
            alias: Some(Cow::Borrowed(value.0.as_str())),
            architecture: None,
            model: None,
            status: None,
        }
    }
}

impl<'a> From<&'a rs4a_vlt::responses::Device> for Device<'a> {
    fn from(value: &'a rs4a_vlt::responses::Device) -> Self {
        let rs4a_vlt::responses::Device {
            architecture,
            id,
            model,
            status,
            ..
        } = value;
        Self {
            alias: Some(Cow::Owned(format!("vlt-{id}"))),
            architecture: Some(*architecture),
            model: Some(model.as_ref()),
            status: Some(*status),
        }
    }
}
impl<'a> From<&'a Loan> for Device<'a> {
    fn from(value: &'a Loan) -> Self {
        let loanable_id = value.loanable.id;
        Self {
            alias: Some(Cow::Owned(format!("vlt-{loanable_id}"))),
            architecture: None,
            model: None,
            status: None,
        }
    }
}

impl<'a> From<&'a rs4a_dut::Device> for Device<'a> {
    fn from(_value: &'a rs4a_dut::Device) -> Self {
        Self {
            alias: None,
            architecture: None,
            model: None,
            status: None,
        }
    }
}

#[derive(Clone, Debug, clap::Parser)]
pub struct SearchCommand {
    /// An alias for the device unique within the inventory.
    #[arg(long)]
    alias: Option<String>,
    #[arg(long, short)]
    architecture: Option<DeviceArchitecture>,
    #[arg(long, short)]
    model: Option<String>,
    #[arg(long, short)]
    status: Option<DeviceStatus>,
}

pub(crate) struct SearchFilter {
    alias: Option<glob::Pattern>,
    model: Option<glob::Pattern>,
    architecture: Option<DeviceArchitecture>,
    status: Option<DeviceStatus>,
}

impl SearchFilter {
    pub(crate) fn matches(&self, d: Device) -> bool {
        let Device {
            alias,
            architecture,
            model,
            status,
        } = d;

        if let Some(p) = self.alias.as_ref() {
            if let Some(alias) = alias {
                if !p.matches(&alias.to_lowercase()) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(p) = self.model.as_ref() {
            if let Some(model) = model {
                if !p.matches(&model.to_lowercase()) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(a) = self.architecture {
            if let Some(architecture) = architecture {
                if a != architecture {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(s) = self.status {
            if let Some(status) = status {
                if s != status {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

impl SearchCommand {
    pub(crate) fn into_filter(self) -> anyhow::Result<SearchFilter> {
        let Self {
            alias,
            architecture,
            model,
            status,
        } = self;
        let alias = alias
            .map(|s| glob::Pattern::new(s.to_lowercase().as_str()))
            .transpose()?;
        let model = model
            .map(|s| glob::Pattern::new(s.to_lowercase().as_str()))
            .transpose()?;
        Ok(SearchFilter {
            alias,
            model,
            architecture,
            status,
        })
    }

    pub async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<()> {
        let client = db_vlt::client(db, offline)
            .await?
            .context("VLT is not configured, skipping VLT devices")?;
        let filter = self.into_filter()?;

        // Loaned devices are not filtered because the loans does not include enough information,
        // so that would have to be retrieved with additional calls. Furthermore, there's probably
        // only a few loaned devices which are probably all relevant and the devices document
        // confusingly omits loaned devices.
        // TODO: Consider filtering loaned devices
        let loaned = requests::loans().send(&client).await?;

        let mut other = requests::devices()
            .send(&client)
            .await?
            .into_iter()
            .filter(|d| filter.matches(Device::from(d)))
            .collect::<Vec<_>>();
        other.sort_by_key(|d| d.id.as_u16());

        let mut aliases = vec!["ALIAS".to_string()];
        let mut models = vec!["MODEL".to_string()];
        let mut architectures = vec!["ARCHITECTURE".to_string()];
        let mut statuses = vec!["STATUS".to_string()];

        for loan in loaned {
            let (alias, device) = try_device_from_loan(loan)?;
            let db::Device { model, .. } = device;
            aliases.push(alias);
            models.push(model.unwrap_or_default());
            architectures.push("".to_string());
            statuses.push("loaned".to_string());
        }

        for device in other {
            let rs4a_vlt::responses::Device {
                architecture,
                id,
                model,
                status,
                ..
            } = device;
            aliases.push(format!("vlt-{id}"));
            models.push(model.to_string());
            architectures.push(
                serde_json::to_string(&architecture)
                    .unwrap()
                    .trim_matches('"')
                    .to_string(),
            );
            statuses.push(status.as_str().to_string());
        }

        let aliases_width = 1 + aliases.iter().map(|s| s.len()).max().unwrap();
        let models_width = 1 + models.iter().map(|s| s.len()).max().unwrap();
        let architectures_width = 1 + architectures.iter().map(|s| s.len()).max().unwrap();
        let statuses_width = 1 + statuses.iter().map(|s| s.len()).max().unwrap();

        for (((alias, model), architecture), status) in aliases
            .into_iter()
            .zip(models.into_iter())
            .zip(architectures.into_iter())
            .zip(statuses.into_iter())
        {
            println!("{alias:aliases_width$} {model:models_width$} {architecture:architectures_width$} {status:statuses_width$}");
        }
        Ok(())
    }
}

// https://195.60.68.14:41095/axis-cgi/param.cgi?action=list&group=Brand.Brand&_=1760380912383

use std::{borrow::Cow, collections::HashMap};

use anyhow::Context;

use crate::Client;

pub struct ListParamsRequest {
    group: Option<Cow<'static, str>>,
}

impl ListParamsRequest {
    pub fn group(mut self, group: impl Into<Cow<'static, str>>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub async fn send(self, client: &Client) -> anyhow::Result<HashMap<String, String>> {
        let Self { group } = self;
        let mut query: HashMap<&'static str, Cow<'static, str>> = HashMap::new();
        query.insert("action", "list".into());
        if let Some(group) = group {
            query.insert("group", group);
        }
        let resp = client
            .get("axis-cgi/prod_brand_info/getbrand.cgi")?
            .query(&query)
            .send()
            .await?
            .error_for_status()?;

        let status = resp.status();
        let text = resp.text().await?;

        let mut params = HashMap::new();
        for line in text.lines() {
            let (key, value) = line.split_once("=").with_context(|| {
                format!("Line {line} from {status} response has no '=' separator")
            })?;
            params.insert(key.to_string(), value.to_string());
        }

        Ok(params)
    }
}

pub fn list_params() -> ListParamsRequest {
    ListParamsRequest { group: None }
}

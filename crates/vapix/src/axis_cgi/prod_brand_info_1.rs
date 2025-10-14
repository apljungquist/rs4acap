// https://195.60.68.14:41095/axis-cgi/prod_brand_info/getbrand.cgi?timestamp=1760380912886

use anyhow::Context;
use serde::Deserialize;

use crate::Client;

// TODO: Consider tightening field types

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Brand {
    pub brand: String,
    pub prod_type: String,
    pub prod_short_name: String,
    pub prod_nbr: String,
    pub prod_full_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct GetBrandResponse {
    brand: Brand,
}

pub struct GetBrandRequest;

impl GetBrandRequest {
    pub async fn send(self, client: &Client) -> anyhow::Result<Brand> {
        let resp = client
            .get("axis-cgi/prod_brand_info/getbrand.cgi")?
            .send()
            .await?
            .error_for_status()?;

        let status = resp.status();
        let text = resp.text().await?;

        serde_json::from_str::<GetBrandResponse>(&text)
            .with_context(|| format!("Status: '{}'", status))
            .map(|b| b.brand)
    }
}

pub fn get_brand() -> GetBrandRequest {
    GetBrandRequest
}

use serde::Deserialize;

use crate::soap::{parse_data, SoapResponse};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AddActionConfigurationResponse {
    #[serde(rename = "ConfigurationID")]
    pub configuration_id: u16,
}

impl SoapResponse for AddActionConfigurationResponse {
    fn from_envelope(text: &str) -> anyhow::Result<Self> {
        parse_data(text)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetActionConfigurationsResponse {
    pub action_configurations: ActionConfigurations,
}

impl SoapResponse for GetActionConfigurationsResponse {
    fn from_envelope(text: &str) -> anyhow::Result<Self> {
        parse_data(text)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ActionConfigurations {
    pub action_configuration: Vec<ActionConfiguration>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ActionConfiguration {
    #[serde(rename = "ConfigurationID")]
    pub configuration_id: u32,
    pub name: String,
    pub template_token: String,
    pub parameters: Parameters,
}

#[derive(Debug, Deserialize)]
pub struct Parameters {
    #[serde(rename = "Parameter")]
    pub parameter: Vec<Parameter>,
}

#[derive(Debug, Deserialize)]
pub struct Parameter {
    #[serde(rename = "@Name")]
    pub name: String,

    #[serde(rename = "@Value")]
    pub value: String,
}

#[cfg(test)]
mod tests {

    use crate::{
        services::action1::action_configurations::AddActionConfigurationResponse, soap::parse_data,
    };

    #[test]
    fn can_deserialize_add_action_configuration_response() {
        let text = include_str!("examples/add_action_configuration_response.xml");
        let data: AddActionConfigurationResponse = parse_data(text).unwrap();
        assert_eq!(1, data.configuration_id);
    }
}

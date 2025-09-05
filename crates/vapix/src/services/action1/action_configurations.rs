use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AddActionConfigurationResponse {
    #[serde(rename = "ConfigurationID")]
    pub configuration_id: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetActionConfigurationsResponse {
    pub action_configurations: ActionConfigurations,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ActionConfigurations {
    #[serde(default)]
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
    use super::*;
    use crate::soap::parse_soap;

    #[test]
    fn can_deserialize_add_action_configuration_response() {
        let text = include_str!("examples/add_action_configuration_response.xml");
        let data = parse_soap::<AddActionConfigurationResponse>(text).unwrap();
        assert_eq!(1, data.configuration_id);
    }

    #[test]
    fn can_deserialize_get_action_configurations_response() {
        let text = include_str!("examples/get_action_configurations_200_response.xml");
        let data = parse_soap::<GetActionConfigurationsResponse>(text).unwrap();
        assert_eq!(0, data.action_configurations.action_configuration.len());
    }
}

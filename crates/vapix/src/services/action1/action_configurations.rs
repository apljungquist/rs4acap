use serde::{Deserialize, Serialize};

use crate::{
    soap::SimpleRequest,
    soap_http::{SoapHttpRequest, SoapRequest},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AddActionConfigurationResponse {
    #[serde(rename = "ConfigurationID")]
    pub configuration_id: u16,
}

pub struct AddActionConfigurationRequest {
    name: Option<String>,
    template_token: String,
    parameters: Parameters,
}

// TODO: Consider enabling typed templates to give users early feedback about missing params
impl AddActionConfigurationRequest {
    pub(crate) fn new(template_token: &str) -> AddActionConfigurationRequest {
        Self {
            name: None,
            template_token: template_token.to_string(),
            parameters: Parameters {
                parameter: Vec::new(),
            },
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn param(mut self, name: &str, value: &str) -> Self {
        self.parameters.parameter.push(Parameter {
            name: name.to_string(),
            value: value.to_string(),
        });
        self
    }
}

impl SoapRequest for AddActionConfigurationRequest {
    fn to_envelope(self) -> anyhow::Result<String> {
        let Self {
            name,
            template_token,
            parameters,
        } = self;
        let mut params = String::new();
        params.push_str(r#"<NewActionConfiguration>"#);
        if let Some(name) = name {
            params.push_str(r#"<Name>"#);
            params.push_str(&name);
            params.push_str(r#"</Name>"#);
        }
        params.push_str(r#"<TemplateToken>"#);
        params.push_str(&template_token);
        params.push_str(r#"</TemplateToken>"#);
        params.push_str(&quick_xml::se::to_string(&parameters)?);
        params.push_str(r#"</NewActionConfiguration>"#);
        SimpleRequest::<()>::new(
            "http://www.axis.com/vapix/ws/action1",
            "AddActionConfiguration",
        )
        .params(params)
        .to_envelope()
    }
}

impl SoapHttpRequest for AddActionConfigurationRequest {
    type Data = AddActionConfigurationResponse;
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

#[derive(Debug, Deserialize, Serialize)]
pub struct Parameters {
    #[serde(rename = "Parameter")]
    pub parameter: Vec<Parameter>,
}

#[derive(Debug, Deserialize, Serialize)]
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

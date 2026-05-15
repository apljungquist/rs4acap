use serde::{Deserialize, Serialize};

use crate::{
    http::{HttpClient, Request},
    protocol_helpers::{
        http::Error,
        soap,
        soap::{parse_empty_response_or_fault, Fault},
        soap_http,
        soap_http::SoapResponse,
    },
};

const PATH: &str = "vapix/services";

const NAMESPACE: &str = "http://www.axis.com/vapix/ws/action1";

#[derive(Debug, Deserialize, Serialize)]
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
    pub fn new(template_token: &str) -> AddActionConfigurationRequest {
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

    fn build_params(self) -> anyhow::Result<String> {
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
        Ok(params)
    }

    pub fn try_into_envelope(self) -> anyhow::Result<String> {
        Ok(soap::envelope(
            NAMESPACE,
            "AddActionConfiguration",
            Some(&self.build_params()?),
        ))
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<AddActionConfigurationResponse, Error<Fault>> {
        let envelope = self.try_into_envelope().map_err(Error::Request)?;
        let request = Request::new(reqwest::Method::POST, PATH.to_string()).soap(envelope);
        soap_http::send_request(client, request).await
    }
}

pub struct RemoveActionConfigurationRequest {
    configuration_id: u16,
}

impl RemoveActionConfigurationRequest {
    pub fn new(configuration_id: u16) -> Self {
        Self { configuration_id }
    }

    pub fn into_envelope(self) -> String {
        let mut params = String::new();
        params.push_str(r#"<ConfigurationID>"#);
        params.push_str(&self.configuration_id.to_string());
        params.push_str(r#"</ConfigurationID>"#);
        soap::envelope(NAMESPACE, "RemoveActionConfiguration", Some(&params))
    }

    pub async fn send(self, client: &(impl HttpClient + Sync)) -> Result<(), Error<Fault>> {
        let request =
            Request::new(reqwest::Method::POST, PATH.to_string()).soap(self.into_envelope());
        let RemoveActionConfigurationResponse = soap_http::send_request(client, request).await?;
        Ok(())
    }
}

struct RemoveActionConfigurationResponse;

impl SoapResponse for RemoveActionConfigurationResponse {
    fn from_envelope(s: &str) -> anyhow::Result<Result<Self, Fault>> {
        Ok(parse_empty_response_or_fault(s, "RemoveActionConfigurationResponse")?.map(|()| Self))
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetActionConfigurationsResponse {
    pub action_configurations: ActionConfigurations,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ActionConfigurations {
    #[serde(default)]
    pub action_configuration: Vec<ActionConfiguration>,
}

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Default)]
pub struct GetActionConfigurationsRequest;

impl GetActionConfigurationsRequest {
    pub fn new() -> Self {
        Self
    }

    pub fn into_envelope(self) -> String {
        soap::envelope(NAMESPACE, "GetActionConfigurations", None)
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<GetActionConfigurationsResponse, Error<Fault>> {
        let request =
            Request::new(reqwest::Method::POST, PATH.to_string()).soap(self.into_envelope());
        soap_http::send_request(client, request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol_helpers::soap::parse_soap;

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

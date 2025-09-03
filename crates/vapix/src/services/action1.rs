//! The [action service API].
//!
//! [action service API]: https://developer.axis.com/vapix/network-video/event-and-action-services
mod action_configurations;
mod action_rules;

pub use action_configurations::GetActionConfigurationsResponse;
pub use action_rules::GetActionRulesResponse;

use crate::{
    services::action1::{
        action_configurations::AddActionConfigurationResponse, action_rules::AddActionRuleResponse,
    },
    soap::{Body, RequestBuilder},
    Client,
};

pub struct Action1 {
    client: Client,
}

impl Action1 {
    pub fn add_action_configuration(self) -> RequestBuilder<AddActionConfigurationResponse> {
        RequestBuilder {
            client: self.client,
            path: "vapix/services",
            body: Body::new(
                "http://www.axis.com/vapix/ws/action1",
                "AddActionConfiguration",
            ),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn add_action_rule(self) -> RequestBuilder<AddActionRuleResponse> {
        RequestBuilder {
            client: self.client,
            path: "vapix/services",
            body: Body::new("http://www.axis.com/vapix/ws/action1", "AddActionRule"),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn get_action_configurations(self) -> RequestBuilder<GetActionConfigurationsResponse> {
        RequestBuilder {
            client: self.client,
            path: "vapix/services",
            body: Body::new(
                "http://www.axis.com/vapix/ws/action1",
                "GetActionConfigurations",
            ),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn get_action_rules(self) -> RequestBuilder<GetActionRulesResponse> {
        RequestBuilder {
            client: self.client,
            path: "vapix/services",
            body: Body::new("http://www.axis.com/vapix/ws/action1", "GetActionRules"),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl Client {
    pub fn action1(&self) -> Action1 {
        Action1 {
            client: self.clone(),
        }
    }
}

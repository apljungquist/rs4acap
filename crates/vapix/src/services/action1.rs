//! The [action service API].
//!
//! [action service API]: https://developer.axis.com/vapix/network-video/event-and-action-services
mod action_configurations;
mod action_rules;

pub use action_configurations::{
    AddActionConfigurationRequest, AddActionConfigurationResponse, GetActionConfigurationsRequest,
    GetActionConfigurationsResponse,
};
pub use action_rules::{
    AddActionRuleRequest, AddActionRuleResponse, Condition, GetActionRulesRequest,
    GetActionRulesResponse,
};

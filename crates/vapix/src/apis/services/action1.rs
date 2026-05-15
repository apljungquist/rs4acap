//! The [action service API].
//!
//! [action service API]: https://developer.axis.com/vapix/network-video/event-and-action-services
mod action_configurations;
mod action_rules;
mod fault;

pub use action_configurations::{
    AddActionConfigurationRequest, AddActionConfigurationResponse, GetActionConfigurationsRequest,
    GetActionConfigurationsResponse, RemoveActionConfigurationRequest,
};
pub use action_rules::{
    ActionRule, AddActionRuleRequest, AddActionRuleResponse, Condition, GetActionRulesRequest,
    GetActionRulesResponse, MessageContent, RemoveActionRuleRequest, TopicExpression,
};
pub use fault::FaultDetail;

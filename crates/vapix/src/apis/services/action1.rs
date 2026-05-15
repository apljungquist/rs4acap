//! The [action service API].
//!
//! # Type-vs-string conventions
//!
//! Wire-primitive response fields (boolean, integer) are exposed as the typed Rust primitive
//! (e.g. `ActionRule.enabled: bool`). Human-readable text and free-form identifiers stay as
//! `String` (e.g. `ActionRule.name`, `ActionConfiguration.template_token`). When the WSDL
//! declares an enumerated value, model it as a Rust `enum` with `FromStr`.
//!
//! [action service API]: https://developer.axis.com/vapix/network-video/event-and-action-services
mod action_configurations;
mod action_rules;

pub use action_configurations::{
    AddActionConfigurationRequest, AddActionConfigurationResponse, GetActionConfigurationsRequest,
    GetActionConfigurationsResponse, RemoveActionConfigurationRequest,
};
pub use action_rules::{
    ActionRule, AddActionRuleRequest, AddActionRuleResponse, Condition, GetActionRulesRequest,
    GetActionRulesResponse, MessageContent, RemoveActionRuleRequest, TopicExpression,
};

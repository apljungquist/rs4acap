//! Request builders for the APIs that have pre-built bindings.
//!
//! Each submodule re-exports the request builders for one API, making it easy to discover what is
//! supported without navigating the full type surface of the crate.

pub mod api_discovery_1 {
    pub use crate::apis::api_discovery_1::{GetApiListRequest, GetSupportedVersionsRequest};
}

pub mod discover {
    pub use crate::apis::discover::DiscoverRequest;
}

pub mod applications_config {
    pub use crate::apis::applications_config::ApplicationConfigRequest;
}

pub mod action_1 {
    pub use crate::apis::action1::{
        AddActionConfigurationRequest, AddActionRuleRequest, GetActionConfigurationsRequest,
        GetActionRulesRequest,
    };
}
pub mod basic_device_info_1 {
    pub use crate::apis::basic_device_info_1::{
        GetAllPropertiesRequest, GetAllUnrestrictedPropertiesRequest,
    };
}

pub mod event_1 {
    pub use crate::apis::event1::GetEventInstancesRequest;
}

pub mod jpg_3 {
    pub use crate::apis::jpg_3::GetImageRequest;
}

pub mod remote_object_storage_1_beta {
    pub use crate::apis::remote_object_storage_1_beta::{
        CreateDestinationRequest, DeleteDestinationRequest, ListDestinationsRequest,
        UpdateDestinationRequest,
    };
}

pub mod recording_group_1 {
    pub use crate::apis::recording_group_1::CreateRecordingGroupsRequest;
}

pub mod siren_and_light_2_alpha {
    pub use crate::apis::siren_and_light_2_alpha::{
        GetMaintenanceModeRequest, StartMaintenanceModeRequest, StopMaintenanceModeRequest,
    };
}

pub mod ssh_1 {
    pub use crate::apis::ssh_1::{AddUserRequest, DeleteUserRequest, SetUserRequest};
}

pub mod system_ready_1 {
    pub use crate::apis::system_ready_1::SystemReadyRequest;
}

pub mod firmware_management_1 {
    pub use crate::apis::firmware_management_1::{FactoryDefaultRequest, UpgradeRequest};
}

pub mod network_settings_1 {
    pub use crate::apis::network_settings_1::{
        GetNetworkInfoRequest, SetGlobalProxyConfigurationRequest,
    };
}

pub mod parameter_management {
    pub use crate::apis::parameter_management::{ListRequest, UpdateRequest};
}

pub mod pwdgrp {
    pub use crate::apis::pwdgrp::{AddUserRequest, RemoveUserRequest};
}

pub mod action_1 {
    pub use crate::action1::{
        action_configurations::{AddActionConfigurationRequest, RemoveActionConfigurationRequest},
        action_rules::{AddActionRuleRequest, RemoveActionRuleRequest},
        add_action_configuration, add_action_rule, get_action_configurations, get_action_rules,
        remove_action_configuration, remove_action_rule,
    };
}
pub mod basic_device_info_1 {
    pub use crate::basic_device_info_1::{get_all_properties, get_all_unrestricted_properties};
}

pub mod event_1 {
    pub use crate::event1::get_event_instances;
}

pub mod jpg_3 {
    pub use crate::axis_cgi::jpg_3::get_image;
}

pub mod remote_object_storage_1 {
    #[allow(deprecated)]
    pub use crate::config::remote_object_storage_1::create_destinations;
}

pub mod remote_object_storage_1_beta {
    pub use crate::config::remote_object_storage_1_beta::{
        CreateDestinationRequest, DeleteDestinationRequest, ListDestinationsRequest,
        UpdateDestinationRequest,
    };
}

pub mod recording_group_1 {
    pub use crate::config::recording_group_1::create_recording_groups;
}

pub mod recording_group_2 {
    pub use crate::config::recording_group_2::{
        CreateRecordingGroupRequest, DeleteRecordingGroupRequest, ListRecordingGroupsRequest,
    };
}

pub mod siren_and_light_2_alpha {
    pub use crate::config::siren_and_light_2_alpha::{
        GetMaintenanceModeRequest, StartMaintenanceModeRequest, StopMaintenanceModeRequest,
    };
}

pub mod ssh_1 {
    pub use crate::config::ssh_1::{add_user, set_user};
}

pub mod system_ready_1 {
    pub use crate::system_ready_1::{system_ready, SystemreadyData};
}

pub mod firmware_management_1 {
    pub use crate::firmware_management_1::{factory_default, FactoryDefaultMode};
}

pub mod parameter_management {
    pub use crate::parameter_management::update;
}

pub mod pwdgrp {
    pub use crate::pwdgrp::{add_user, Group, Role};
}

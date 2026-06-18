use crate::AreaConfig;
use crate::AreaConfigStabby;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_ADD_AREA: &str = "area.add";

#[derive(Debug, Clone)]
pub struct AddAreaMessage {
    pub area_id: String,
    pub area_config: AreaConfig,
}

/// ABI-stable version of `AddAreaMessage` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct AddAreaMessageStabby {
    pub area_id: stabby::string::String,
    pub area_config: AreaConfigStabby,
}

impl From<AddAreaMessage> for AddAreaMessageStabby {
    fn from(value: AddAreaMessage) -> Self {
        Self {
            area_id: value.area_id.into(),
            area_config: value.area_config.into(),
        }
    }
}

impl From<AddAreaMessageStabby> for AddAreaMessage {
    fn from(value: AddAreaMessageStabby) -> Self {
        Self {
            area_id: value.area_id.to_string(),
            area_config: value.area_config.into(),
        }
    }
}

impl AddAreaMessage {
    pub fn new(area_id: &str, area_config: AreaConfig) -> Self {
        Self {
            area_id: area_id.to_string(),
            area_config,
        }
    }
}

impl TypedMessage for AddAreaMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_model_area::AddAreaMessageStabby");
}

impl MessageTopic for AddAreaMessageStabby {
    fn topic() -> &'static str {
        TOPIC_ADD_AREA
    }
}

impl SharedMessage for AddAreaMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_ADD_AREA
    }
}

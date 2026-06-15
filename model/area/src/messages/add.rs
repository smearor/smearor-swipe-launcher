use crate::AreaConfig;
use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;

pub const TOPIC_ADD_AREA: &str = "area.add";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAreaMessage {
    pub area_id: String,
    pub area_config: AreaConfig,
}

impl AddAreaMessage {
    pub fn new(area_id: &str, area_config: AreaConfig) -> Self {
        Self {
            area_id: area_id.to_string(),
            area_config,
        }
    }
}

impl MessageTopic for AddAreaMessage {
    fn topic() -> &'static str {
        TOPIC_ADD_AREA
    }
}

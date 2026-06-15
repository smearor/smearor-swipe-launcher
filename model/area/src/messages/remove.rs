use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;

pub const TOPIC_REMOVE_AREA: &str = "area.remove";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveAreaMessage {
    pub area_id: String,
}

impl RemoveAreaMessage {
    pub fn new(area_id: &str) -> Self {
        Self { area_id: area_id.to_string() }
    }
}

impl MessageTopic for RemoveAreaMessage {
    fn topic() -> &'static str {
        TOPIC_REMOVE_AREA
    }
}

use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;

pub const TOPIC_OPEN_AREA: &str = "area.open";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAreaMessage {
    pub area_id: String,
}

impl OpenAreaMessage {
    pub fn new(area_id: &str) -> Self {
        Self { area_id: area_id.to_string() }
    }
}

impl MessageTopic for OpenAreaMessage {
    fn topic() -> &'static str {
        TOPIC_OPEN_AREA
    }
}

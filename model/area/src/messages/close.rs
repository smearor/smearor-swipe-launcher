use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;

pub const TOPIC_CLOSE_AREA: &str = "area.close";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseAreaMessage {
    pub area_id: String,
}

impl CloseAreaMessage {
    pub fn new(area_id: &str) -> Self {
        Self { area_id: area_id.to_string() }
    }
}

impl MessageTopic for CloseAreaMessage {
    fn topic() -> &'static str {
        TOPIC_CLOSE_AREA
    }
}

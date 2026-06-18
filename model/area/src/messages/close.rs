use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_CLOSE_AREA: &str = "area.close";

#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct CloseAreaMessage {
    pub area_id: stabby::string::String,
}

impl CloseAreaMessage {
    pub fn new(area_id: &str) -> Self {
        Self { area_id: area_id.into() }
    }
}

impl TypedMessage for CloseAreaMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_area::CloseAreaMessage");
}

impl MessageTopic for CloseAreaMessage {
    fn topic() -> &'static str {
        TOPIC_CLOSE_AREA
    }
}

impl SharedMessage for CloseAreaMessage {
    fn topic(&self) -> &'static str {
        TOPIC_CLOSE_AREA
    }
}

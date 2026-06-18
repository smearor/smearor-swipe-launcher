use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_OPEN_AREA: &str = "area.open";

#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct OpenAreaMessage {
    pub area_id: stabby::string::String,
}

impl OpenAreaMessage {
    pub fn new(area_id: &str) -> Self {
        Self { area_id: area_id.into() }
    }
}

impl TypedMessage for OpenAreaMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_area::OpenAreaMessage");
}

impl MessageTopic for OpenAreaMessage {
    fn topic() -> &'static str {
        TOPIC_OPEN_AREA
    }
}

impl SharedMessage for OpenAreaMessage {
    fn topic(&self) -> &'static str {
        TOPIC_OPEN_AREA
    }
}

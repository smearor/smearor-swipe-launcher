use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_REMOVE_AREA: &str = "area.remove";

#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct RemoveAreaMessage {
    pub area_id: stabby::string::String,
}

impl RemoveAreaMessage {
    pub fn new(area_id: &str) -> Self {
        Self { area_id: area_id.into() }
    }
}

impl TypedMessage for RemoveAreaMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_area::RemoveAreaMessage");
}

impl MessageTopic for RemoveAreaMessage {
    fn topic() -> &'static str {
        TOPIC_REMOVE_AREA
    }
}

impl SharedMessage for RemoveAreaMessage {
    fn topic(&self) -> &'static str {
        TOPIC_REMOVE_AREA
    }
}

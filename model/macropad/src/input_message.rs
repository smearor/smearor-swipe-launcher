use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::topics::TOPIC_MACROPAD_INPUT;

/// A message representing an input event from a MacroPad device.
///
/// Sent by MacroPad services (Stream Deck, Loupedeck) when a button is pressed
/// or released.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MacroPadInputMessage {
    /// Serial number or unique identifier of the source device.
    pub device_id: stabby::string::String,
    /// Instance ID associated with the device.
    pub instance_id: stabby::string::String,
    /// Button index that changed (0-based).
    pub button_index: u8,
    /// True if the button was pressed, false if released.
    pub pressed: bool,
}

impl MacroPadInputMessage {
    /// Create a new input message.
    pub fn new(device_id: &str, instance_id: &str, button_index: u8, pressed: bool) -> Self {
        Self {
            device_id: device_id.into(),
            instance_id: instance_id.into(),
            button_index,
            pressed,
        }
    }
}

impl TypedMessage for MacroPadInputMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_macropad::MacroPadInputMessage");
}

impl MessageTopic for MacroPadInputMessage {
    fn topic() -> &'static str {
        TOPIC_MACROPAD_INPUT
    }
}

impl SharedMessage for MacroPadInputMessage {
    fn topic(&self) -> &'static str {
        TOPIC_MACROPAD_INPUT
    }
}

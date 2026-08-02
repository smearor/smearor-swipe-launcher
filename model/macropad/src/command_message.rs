use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::topics::TOPIC_MACROPAD_COMMAND;

/// The type of command to send to a MacroPad device.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroPadCommandType {
    /// Set the display brightness (0-100).
    #[default]
    SetBrightness,
    /// Clear all button images.
    ClearAllButtons,
    /// Clear a specific button image.
    ClearButton,
    /// Set a button image from raw RGBA pixel data.
    SetButtonImage,
    /// Reset the device to its default state.
    Reset,
}

/// A command to send to a MacroPad device.
///
/// Sent by the host or other instances to control MacroPad hardware:
/// set brightness, clear buttons, or set button images.
///
/// The `command_type` field determines which data fields are relevant:
/// - `SetBrightness`: uses `percent`
/// - `ClearButton`: uses `button_index`
/// - `SetButtonImage`: uses `button_index`, `width`, `height`, and `pixels`
/// - `ClearAllButtons` and `Reset`: no data fields
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MacroPadCommand {
    /// The type of command to execute.
    pub command_type: MacroPadCommandType,
    /// Brightness percentage (0-100). Used by `SetBrightness`.
    pub percent: u8,
    /// The button index (0-based). Used by `ClearButton`, `SetButtonImage`.
    pub button_index: u8,
    /// Image width in pixels. Used by `SetButtonImage`.
    pub width: u32,
    /// Image height in pixels. Used by `SetButtonImage`.
    pub height: u32,
    /// Raw RGBA pixel data. Used by `SetButtonImage`.
    pub pixels: stabby::vec::Vec<u8>,
}

impl MacroPadCommand {
    /// Create a `SetBrightness` command.
    pub fn set_brightness(percent: u8) -> Self {
        Self {
            command_type: MacroPadCommandType::SetBrightness,
            percent,
            button_index: 0,
            width: 0,
            height: 0,
            pixels: stabby::vec::Vec::new(),
        }
    }

    /// Create a `ClearAllButtons` command.
    pub fn clear_all_buttons() -> Self {
        Self {
            command_type: MacroPadCommandType::ClearAllButtons,
            percent: 0,
            button_index: 0,
            width: 0,
            height: 0,
            pixels: stabby::vec::Vec::new(),
        }
    }

    /// Create a `ClearButton` command.
    pub fn clear_button(button_index: u8) -> Self {
        Self {
            command_type: MacroPadCommandType::ClearButton,
            percent: 0,
            button_index,
            width: 0,
            height: 0,
            pixels: stabby::vec::Vec::new(),
        }
    }

    /// Create a `SetButtonImage` command.
    pub fn set_button_image(button_index: u8, width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self {
            command_type: MacroPadCommandType::SetButtonImage,
            percent: 0,
            button_index,
            width,
            height,
            pixels: stabby::vec::Vec::from(pixels.as_slice()),
        }
    }

    /// Create a `Reset` command.
    pub fn reset() -> Self {
        Self {
            command_type: MacroPadCommandType::Reset,
            percent: 0,
            button_index: 0,
            width: 0,
            height: 0,
            pixels: stabby::vec::Vec::new(),
        }
    }
}

/// A message containing a command for a MacroPad device.
///
/// Sent by the host or other instances to control MacroPad hardware.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MacroPadCommandMessage {
    /// Device identifier (empty = all devices managed by the service).
    pub device_id: stabby::string::String,
    /// The command to execute.
    pub command: MacroPadCommand,
}

impl MacroPadCommandMessage {
    /// Create a new command message.
    pub fn new(device_id: &str, command: MacroPadCommand) -> Self {
        Self {
            device_id: device_id.into(),
            command,
        }
    }
}

impl TypedMessage for MacroPadCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_macropad::MacroPadCommandMessage");
}

impl MessageTopic for MacroPadCommandMessage {
    fn topic() -> &'static str {
        TOPIC_MACROPAD_COMMAND
    }
}

impl SharedMessage for MacroPadCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_MACROPAD_COMMAND
    }
}

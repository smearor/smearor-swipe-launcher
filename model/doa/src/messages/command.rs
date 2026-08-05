use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_COMMAND: &str = "service.doa.command";

/// Actions that can be sent to the DoA service.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoaCommandAction {
    /// Restart the USB connection and resume polling.
    #[default]
    Reconnect,
    /// Set the polling interval in milliseconds. `value` = new interval.
    SetPollInterval,
    /// Pause DoA polling (stop reading from the device).
    Pause,
    /// Resume DoA polling (continue reading from the device).
    Resume,
}

/// Command message sent to the DoA service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DoaCommandMessage {
    /// The action to perform.
    pub action: DoaCommandAction,
    /// Target state for the action. Semantics depend on `action`:
    /// - `SetPollInterval`: new interval in milliseconds.
    pub value: u64,
}

impl TypedMessage for DoaCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_doa_model::DoaCommandMessage");
}

impl MessageTopic for DoaCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl SharedMessage for DoaCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_COMMAND
    }
}

#[cfg(test)]
mod tests {
    use super::DoaCommandAction;
    use super::DoaCommandMessage;
    use super::TOPIC_COMMAND;
    use smearor_swipe_launcher_plugin_api::MessageTopic;
    use smearor_swipe_launcher_plugin_api::SharedMessage;

    #[test]
    fn test_default_action() {
        assert_eq!(DoaCommandAction::default(), DoaCommandAction::Reconnect);
    }

    #[test]
    fn test_default_message() {
        let msg = DoaCommandMessage::default();
        assert_eq!(msg.action, DoaCommandAction::Reconnect);
        assert_eq!(msg.value, 0);
    }

    #[test]
    fn test_serde_round_trip() {
        let msg = DoaCommandMessage {
            action: DoaCommandAction::SetPollInterval,
            value: 200,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: DoaCommandMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.action, DoaCommandAction::SetPollInterval);
        assert_eq!(deserialized.value, 200);
    }

    #[test]
    fn test_serde_action_round_trip() {
        for action in [
            DoaCommandAction::Reconnect,
            DoaCommandAction::SetPollInterval,
            DoaCommandAction::Pause,
            DoaCommandAction::Resume,
        ] {
            let json = serde_json::to_string(&action).unwrap();
            let deserialized: DoaCommandAction = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, action);
        }
    }

    #[test]
    fn test_topic() {
        use smearor_swipe_launcher_plugin_api::MessageTopic;
        assert_eq!(<DoaCommandMessage as MessageTopic>::topic(), "service.doa.command");
        assert_eq!(TOPIC_COMMAND, "service.doa.command");
        let msg = DoaCommandMessage::default();
        assert_eq!(SharedMessage::topic(&msg), "service.doa.command");
    }
}

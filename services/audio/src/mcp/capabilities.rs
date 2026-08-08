use crate::service::AudioService;
use schemars::schema_for;
use smearor_audio_model::AudioSetVolumeArgs;
use smearor_model_mcp::NoArgs;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use tracing::debug;

impl McpCapabilitiesRegistrator for AudioService {
    fn register_mcp_capabilities(&self) {
        if !self.config.mcp_enabled {
            debug!("Audio Service: MCP tool registration disabled by config");
            return;
        }

        let broadcaster = self.get_broadcaster();

        let status_resource = RegisterResourceMessage::new(
            "audio://status",
            "Audio Status",
            "Complete audio status including volume, mute, output devices, and active device.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(status_resource);

        let volume_resource = RegisterResourceMessage::new("audio://volume", "Audio Volume", "Current master volume level (0.0 to 1.0).", "application/json");
        broadcaster.broadcast_message_to_topic(volume_resource);

        let muted_resource = RegisterResourceMessage::new("audio://muted", "Audio Muted", "Current mute status of the default sink.", "application/json");
        broadcaster.broadcast_message_to_topic(muted_resource);

        let active_sink_resource = RegisterResourceMessage::new(
            "audio://active_sink",
            "Active Audio Sink",
            "Currently active output device with name, index, and default flag.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(active_sink_resource);

        let sinks_resource = RegisterResourceMessage::new("audio://sinks", "Audio Output Devices", "List of all available output devices.", "application/json");
        broadcaster.broadcast_message_to_topic(sinks_resource);

        let no_args_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();

        let volume_up_tool = RegisterToolMessage::new("audio_volume_up", "Increases the audio volume by a configured step.", &no_args_schema);
        broadcaster.broadcast_message_to_topic(volume_up_tool);

        let volume_down_tool = RegisterToolMessage::new("audio_volume_down", "Decreases the audio volume by a configured step.", &no_args_schema);
        broadcaster.broadcast_message_to_topic(volume_down_tool);

        let set_volume_schema = serde_json::to_string(&schema_for!(AudioSetVolumeArgs)).unwrap_or_default();
        let set_volume_tool = RegisterToolMessage::new(
            "audio_set_volume",
            "Sets the audio volume to an absolute value. / Lautstärke auf einen absoluten Wert setzen.",
            &set_volume_schema,
        );
        broadcaster.broadcast_message_to_topic(set_volume_tool);

        let toggle_mute_tool = RegisterToolMessage::new("audio_toggle_mute", "Toggles the mute state of the default sink.", &no_args_schema);
        broadcaster.broadcast_message_to_topic(toggle_mute_tool);

        let mute_tool = RegisterToolMessage::new("audio_mute", "Mutes the default audio output sink.", &no_args_schema);
        broadcaster.broadcast_message_to_topic(mute_tool);

        let unmute_tool = RegisterToolMessage::new("audio_unmute", "Unmutes the default audio output sink.", &no_args_schema);
        broadcaster.broadcast_message_to_topic(unmute_tool);

        let next_device_tool = RegisterToolMessage::new("audio_next_device", "Switches to the next available audio output device.", &no_args_schema);
        broadcaster.broadcast_message_to_topic(next_device_tool);

        let previous_device_tool =
            RegisterToolMessage::new("audio_previous_device", "Switches to the previous available audio output device.", &no_args_schema);
        broadcaster.broadcast_message_to_topic(previous_device_tool);

        let refresh_tool = RegisterToolMessage::new("audio_refresh_status", "Force an immediate refresh of the audio status from PulseAudio.", &no_args_schema);
        broadcaster.broadcast_message_to_topic(refresh_tool);

        let prompt = RegisterPromptMessage::with_memory(
            "audio_control_guide",
            "Returns a system prompt with audio control instructions, available tools, resources, and current status snapshot.",
            &no_args_schema,
            "audio volume preference and preferred output device",
            "audio,volume,mute",
        );
        broadcaster.broadcast_message_to_topic(prompt);
    }
}

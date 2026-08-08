use crate::mcp::requests::GetCurrentTimeArgs;
use crate::widget::ClockWidget;
use schemars::schema_for;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for ClockWidget {
    fn register_mcp_capabilities(&self) {
        let schema = serde_json::to_string(&schema_for!(GetCurrentTimeArgs)).unwrap_or_default();
        let tool = RegisterToolMessage::new(
            "get_current_time",
            "Returns the current local time as a structured JSON object with fields: timestamp_iso (ISO 8601), date (DD.MM.YYYY), time (HH:MM), timezone, timezone_label (CET/CEST/etc), is_summer_time (bool), day_of_week, time_of_day_context (late_night/early_morning/morning/noon/afternoon/evening), workday_status (mid_week/weekend).",
            &schema,
        );
        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(tool);

        let resource = RegisterResourceMessage::new(
            "clock://time",
            "current_time",
            "Current time as structured JSON: timestamp_iso, date, time, timezone, timezone_label, is_summer_time, day_of_week, time_of_day_context, workday_status.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(resource);
    }
}

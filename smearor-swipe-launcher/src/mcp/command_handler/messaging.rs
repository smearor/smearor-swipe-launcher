use crate::host::LauncherHost;
use smearor_mcp_server::McpCommand;

use super::common::send_mcp_message;

/// Handle messaging commands (send single or multiple broker messages).
pub(crate) fn handle_messaging_command(host: &LauncherHost, command: McpCommand) {
    match command {
        McpCommand::SendMessage(cmd) => {
            let result = send_mcp_message(host, cmd.params.topic, cmd.params.payload, cmd.params.target_instance_id);
            let _ = cmd.response.send(result);
        }
        McpCommand::SendMultipleMessages(cmd) => {
            let mut seen: Vec<(String, String)> = Vec::new();
            let mut sent_count: u32 = 0;
            let mut skipped_count: u32 = 0;
            for message in cmd.params.messages {
                let payload_key = message.payload.to_string();
                if seen.iter().any(|(t, p)| t == &message.topic && p == &payload_key) {
                    skipped_count += 1;
                    continue;
                }
                seen.push((message.topic.clone(), payload_key));
                let result = send_mcp_message(host, message.topic, message.payload, message.target_instance_id);
                if result.is_ok() {
                    sent_count += 1;
                }
            }
            let result = Ok(format!("{} messages sent, {} duplicates skipped", sent_count, skipped_count));
            let _ = cmd.response.send(result);
        }
        _ => unreachable!("handle_messaging_command received non-messaging command"),
    }
}

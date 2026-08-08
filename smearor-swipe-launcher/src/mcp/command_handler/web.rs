use crate::host::LauncherHost;
use smearor_mcp_server::McpCommand;

/// Handle web server status queries.
pub(crate) fn handle_web_command(host: &LauncherHost, command: McpCommand) {
    match command {
        McpCommand::WebServerStatus(cmd) => {
            let result = host.web_server_status();
            let _ = cmd.response.send(result);
        }
        _ => unreachable!("handle_web_command received non-web command"),
    }
}

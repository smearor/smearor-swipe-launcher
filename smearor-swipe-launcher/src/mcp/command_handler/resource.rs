use crate::host::LauncherHost;
use crate::mcp::resource_reader::resource::read_mcp_resource;
use smearor_mcp_server::McpCommand;

/// Handle resource read commands.
pub(crate) fn handle_resource_command(host: &LauncherHost, command: McpCommand) {
    match command {
        McpCommand::ReadResource(cmd) => {
            let result = read_mcp_resource(host, &cmd.params.uri);
            let _ = cmd.response.send(result);
        }
        _ => unreachable!("handle_resource_command received non-resource command"),
    }
}

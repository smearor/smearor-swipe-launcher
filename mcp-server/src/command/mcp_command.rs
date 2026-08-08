use crate::command::close_area::CloseAreaParams;
use crate::command::focus_area::FocusAreaParams;
use crate::command::get_area_config::GetAreaConfigParams;
use crate::command::invoke_plugin_prompt::InvokePluginPromptParams;
use crate::command::invoke_plugin_resource::InvokePluginResourceParams;
use crate::command::invoke_plugin_tool::InvokePluginToolParams;
use crate::command::list_all_areas::ListAllAreasParams;
use crate::command::list_areas::ListAreasParams;
use crate::command::list_instances::ListInstancesParams;
use crate::command::load_instance::LoadInstanceParams;
use crate::command::open_area::OpenAreaParams;
use crate::command::open_transient_area::OpenTransientAreaParams;
use crate::command::read_resource::ReadResourceParams;
use crate::command::reload_instance::ReloadInstanceParams;
use crate::command::send_message::SendMessageParams;
use crate::command::send_multiple_messages::SendMultipleMessagesParams;
use crate::command::start_instance::StartInstanceParams;
use crate::command::stop_instance::StopInstanceParams;
use crate::command::toggle_area::ToggleAreaParams;
use crate::command::unload_instance::UnloadInstanceParams;
use crate::command::web_server_status::WebServerStatusParams;
use crate::command::wrapper::CommandResponseWrapper;

/// Commands sent from the MCP server to the launcher core.
pub enum McpCommand {
    /// Open an area by ID.
    OpenArea(CommandResponseWrapper<OpenAreaParams>),
    /// Close an area by ID.
    CloseArea(CommandResponseWrapper<CloseAreaParams>),
    /// List all currently managed (opened) areas.
    ListAreas(CommandResponseWrapper<ListAreasParams>),
    /// List all configured areas (including not-yet-opened ones).
    ListAllAreas(CommandResponseWrapper<ListAllAreasParams>),
    /// Open an area as a transient overlay (like a button click).
    OpenTransientArea(CommandResponseWrapper<OpenTransientAreaParams>),
    /// Focus an area by ID.
    FocusArea(CommandResponseWrapper<FocusAreaParams>),
    /// Send a message to a broker topic.
    SendMessage(CommandResponseWrapper<SendMessageParams>),
    /// Send multiple messages to broker topics, with duplicate filtering.
    SendMultipleMessages(CommandResponseWrapper<SendMultipleMessagesParams>),
    /// Read a resource by URI.
    ReadResource(CommandResponseWrapper<ReadResourceParams>),
    /// Toggle the visibility of an area.
    ToggleArea(CommandResponseWrapper<ToggleAreaParams>),
    /// Get the configuration of an area as JSON.
    GetAreaConfig(CommandResponseWrapper<GetAreaConfigParams>),
    /// Invoke a tool registered by a plugin.
    InvokePluginTool(CommandResponseWrapper<InvokePluginToolParams>),
    /// Read a resource registered by a plugin.
    InvokePluginResource(CommandResponseWrapper<InvokePluginResourceParams>),
    /// Invoke a prompt registered by a plugin.
    InvokePluginPrompt(CommandResponseWrapper<InvokePluginPromptParams>),
    /// Dynamically load a new launcher instance.
    LoadInstance(CommandResponseWrapper<LoadInstanceParams>),
    /// Start a loaded (Ready) launcher instance.
    StartInstance(CommandResponseWrapper<StartInstanceParams>),
    /// Stop a running launcher instance (transitions to Ready).
    StopInstance(CommandResponseWrapper<StopInstanceParams>),
    /// Unload a stopped (Ready) launcher instance entirely.
    UnloadInstance(CommandResponseWrapper<UnloadInstanceParams>),
    /// Hot-reload an instance (stop, unload, re-load, restore previous state).
    ReloadInstance(CommandResponseWrapper<ReloadInstanceParams>),
    /// List all running launcher instances.
    ListInstances(CommandResponseWrapper<ListInstancesParams>),
    /// Get the status of the embedded web server.
    WebServerStatus(CommandResponseWrapper<WebServerStatusParams>),
}

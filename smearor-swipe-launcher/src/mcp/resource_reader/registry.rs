use crate::host::LauncherHost;
use crate::mcp::resource_reader::area_buttons_handler::AreaButtonsHandler;
use crate::mcp::resource_reader::area_list_handler::AreaListHandler;
use crate::mcp::resource_reader::area_plugins_handler::AreaPluginsHandler;
use crate::mcp::resource_reader::area_state_handler::AreaStateHandler;
use crate::mcp::resource_reader::plugin_list_handler::PluginListHandler;

/// A handler for a specific MCP resource URI pattern.
pub trait McpResourceHandler: Send + Sync {
    /// Check whether this handler can process the given URI.
    fn uri_matches(&self, uri: &str) -> bool;

    /// Read the resource content for the given URI.
    fn handle(&self, host: &LauncherHost, uri: &str) -> Result<String, String>;
}

/// Registry of MCP resource handlers, checked in registration order.
pub struct McpResourceHandlerRegistry {
    handlers: Vec<Box<dyn McpResourceHandler>>,
}

impl McpResourceHandlerRegistry {
    pub fn new() -> Self {
        Self { handlers: Vec::new() }
    }

    pub fn register(&mut self, handler: Box<dyn McpResourceHandler>) {
        self.handlers.push(handler);
    }

    pub fn read(&self, host: &LauncherHost, uri: &str) -> Result<String, String> {
        for handler in &self.handlers {
            if handler.uri_matches(uri) {
                return handler.handle(host, uri);
            }
        }
        Err(format!("Resource {} not implemented", uri))
    }
}

impl Default for McpResourceHandlerRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(AreaListHandler));
        registry.register(Box::new(AreaStateHandler));
        registry.register(Box::new(AreaPluginsHandler));
        registry.register(Box::new(AreaButtonsHandler));
        registry.register(Box::new(PluginListHandler));
        registry
    }
}

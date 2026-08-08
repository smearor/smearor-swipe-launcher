use tokio::sync::oneshot;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;

/// Generic wrapper that pairs command parameters with a response channel.
#[derive(TypedBuilder)]
pub struct CommandResponseWrapper<T> {
    /// The command parameters.
    pub params: T,
    /// Response channel for the result.
    pub response: oneshot::Sender<Result<String, String>>,
}

/// Trait that maps a parameter type to its corresponding `McpCommand` variant.
/// This enables a single generic `From<CommandResponseWrapper<T>> for McpCommand` implementation.
pub trait McpCommandVariant {
    /// Convert a `CommandResponseWrapper<Self>` into the matching `McpCommand` variant.
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand
    where
        Self: Sized;
}

impl<T: McpCommandVariant> From<CommandResponseWrapper<T>> for McpCommand {
    fn from(wrapper: CommandResponseWrapper<T>) -> Self {
        T::into_command(wrapper)
    }
}

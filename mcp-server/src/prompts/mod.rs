//! MCP prompt definitions and resolution helpers.

mod core;
mod creator;
mod definition;
mod into_sdk_prompt;
mod registered;
mod resolver;
mod result;
mod schema;

pub use definition::PromptDefinition;
pub use definition::PromptHandler;
pub use into_sdk_prompt::IntoSdkPrompt;
pub use into_sdk_prompt::SdkPromptFields;
pub use resolver::PromptResolver;
pub use result::PromptResult;

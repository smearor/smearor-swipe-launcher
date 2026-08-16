pub mod local;
pub mod ollama;
pub mod types;

#[allow(unused_imports)]
pub use local::LocalLlmBackend;
#[allow(unused_imports)]
pub use ollama::OllamaBackend;
pub use types::ChatMessage;
#[allow(unused_imports)]
pub use types::ChatRole;
pub use types::LlmBackend;
pub use types::LlmBackendConfig;
pub use types::LlmBackendType;
pub use types::LlmError;
pub use types::LlmResourceReport;
pub use types::create_backend;

pub mod monitor_identifier;
pub mod window_identifier;
pub mod workspace_identifier;
pub mod workspace_identifier_with_special;

pub use monitor_identifier::HyprlandMonitorIdentifier;
pub use monitor_identifier::HyprlandMonitorIdentifierKind;
pub use window_identifier::HyprlandWindowIdentifier;
pub use workspace_identifier::HyprlandWorkspaceIdentifier;
pub use workspace_identifier_with_special::HyprlandWorkspaceIdentifierKind;
pub use workspace_identifier_with_special::HyprlandWorkspaceIdentifierWithSpecial;

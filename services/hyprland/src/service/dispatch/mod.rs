pub(crate) mod system;
pub(crate) mod toggle;
pub(crate) mod window;
pub(crate) mod workspace;

pub(crate) use system::handle_dispatch_system;
pub(crate) use toggle::handle_dispatch_toggle;
pub(crate) use window::handle_dispatch_window;
pub(crate) use workspace::handle_create_workspace;
pub(crate) use workspace::handle_dispatch_workspace;
pub(crate) use workspace::handle_switch_workspace;

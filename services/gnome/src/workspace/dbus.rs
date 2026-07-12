use zbus::proxy;

/// D-Bus proxy for `org.gnome.Shell.Eval`.
///
/// This interface allows executing JavaScript in the GNOME Shell process.
/// We use it as a fallback for workspace switching and creation.
#[proxy(interface = "org.gnome.Shell", default_service = "org.gnome.Shell", default_path = "/org/gnome/Shell")]
pub trait GnomeShellEval {
    /// Execute JavaScript in the GNOME Shell process.
    fn eval(&self, code: &str) -> zbus::Result<(bool, String)>;
}

/// D-Bus proxy for `org.gnome.Shell.Introspect`.
///
/// This interface provides window information without requiring unsafe mode.
/// The `GetWindows` method returns a map of window IDs to window properties,
/// including the workspace each window is on.
#[proxy(
    interface = "org.gnome.Shell.Introspect",
    default_service = "org.gnome.Shell",
    default_path = "/org/gnome/Shell/Introspect"
)]
pub trait GnomeShellIntrospect {
    /// Get all windows with their properties.
    ///
    /// Returns a map of window ID (as string) to a dictionary of properties.
    /// Key properties include "workspace" (i32), "has-focus" (bool),
    /// "title" (string), and "class" (string).
    fn get_windows(&self) -> zbus::Result<std::collections::HashMap<String, std::collections::HashMap<String, zbus::zvariant::OwnedValue>>>;
}

/// D-Bus proxy for `org.gnome.Mutter.DisplayConfig`.
///
/// This interface provides monitor information for index resolution.
#[proxy(
    interface = "org.gnome.Mutter.DisplayConfig",
    default_service = "org.gnome.Mutter.DisplayConfig",
    default_path = "/org/gnome/Mutter/DisplayConfig"
)]
pub trait MutterDisplayConfig {
    /// Get current monitor resources.
    /// Returns (serial, monitors, logical_monitors, properties).
    fn get_resources(
        &self,
    ) -> zbus::Result<(
        u32,
        Vec<(String, Vec<(String, zbus::zvariant::OwnedValue)>)>,
        Vec<Vec<(i32, i32, u32, bool, Vec<(String, zbus::zvariant::OwnedValue)>)>>,
        Vec<(String, zbus::zvariant::OwnedValue)>,
    )>;

    /// Get current state (newer API, GNOME 46+).
    fn get_current_state(&self) -> zbus::Result<(u32, Vec<zbus::zvariant::OwnedValue>, Vec<zbus::zvariant::OwnedValue>, zbus::zvariant::OwnedValue)>;
}

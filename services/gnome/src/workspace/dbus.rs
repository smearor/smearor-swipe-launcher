use zbus::proxy;

/// D-Bus proxy for `org.gnome.Shell.Eval`.
///
/// This interface allows executing JavaScript in the GNOME Shell process.
/// We use it to query the current workspace index.
#[proxy(interface = "org.gnome.Shell", default_service = "org.gnome.Shell", default_path = "/org/gnome/Shell")]
pub trait GnomeShellEval {
    /// Execute JavaScript in the GNOME Shell process.
    fn eval(&self, code: &str) -> zbus::Result<(bool, String)>;
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

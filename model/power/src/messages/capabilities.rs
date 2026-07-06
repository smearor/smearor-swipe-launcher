/// Capabilities of the system as reported by systemd-logind.
/// Each field corresponds to a `Can*` D-Bus property on `org.freedesktop.login1.Manager`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PowerCapabilities {
    /// Whether the system can be shut down.
    pub can_shutdown: bool,
    /// Whether the system can be rebooted.
    pub can_reboot: bool,
    /// Whether the system can be suspended.
    pub can_suspend: bool,
    /// Whether the system can be hibernated.
    pub can_hibernate: bool,
    /// Whether the system can reboot to firmware/UEFI.
    pub can_reboot_to_firmware: bool,
    /// Whether the session can be locked.
    pub can_lock: bool,
    /// Whether the session can be terminated (log out).
    pub can_logout: bool,
}

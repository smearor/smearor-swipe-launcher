/// All power actions supported by the service.
/// Each variant maps to a specific D-Bus call on `org.freedesktop.login1`.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PowerAction {
    /// Shut the system down gracefully.
    Shutdown,
    /// Restart the system.
    Reboot,
    /// Put the system into RAM sleep (S3).
    Suspend,
    /// Save RAM to disk (swap) and power off.
    Hibernate,
    /// Lock the current session.
    Lock,
    /// Terminate the current session and return to the display manager.
    Logout,
    /// Reboot directly into BIOS/UEFI firmware settings.
    RebootToFirmware,
    /// Cancel a running countdown or scheduled action.
    #[default]
    Cancel,
}

impl PowerAction {
    /// Parses a power action from a string identifier.
    /// Returns `PowerAction::Cancel` for unknown strings.
    pub fn from_str(s: &str) -> Self {
        match s {
            "shutdown" => PowerAction::Shutdown,
            "reboot" => PowerAction::Reboot,
            "suspend" => PowerAction::Suspend,
            "hibernate" => PowerAction::Hibernate,
            "lock" => PowerAction::Lock,
            "logout" => PowerAction::Logout,
            "reboot_to_firmware" => PowerAction::RebootToFirmware,
            _ => PowerAction::Cancel,
        }
    }
}

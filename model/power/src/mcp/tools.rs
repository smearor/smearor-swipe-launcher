use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the power service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerMcpTools {
    /// Execute a power action (shutdown, reboot, suspend, etc.).
    PowerAction,
    /// Schedule a power action with a delay.
    SchedulePowerAction,
    /// Cancel a scheduled power action.
    CancelPowerAction,
    /// Reboot to UEFI firmware.
    RebootToUefi,
}

impl AsRef<str> for PowerMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::PowerAction => "system_power_action",
            Self::SchedulePowerAction => "system_schedule_power_action",
            Self::CancelPowerAction => "system_cancel_power_action",
            Self::RebootToUefi => "system_reboot_to_uefi",
        }
    }
}

impl FromStr for PowerMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "system_power_action" => Ok(Self::PowerAction),
            "system_schedule_power_action" => Ok(Self::SchedulePowerAction),
            "system_cancel_power_action" => Ok(Self::CancelPowerAction),
            "system_reboot_to_uefi" => Ok(Self::RebootToUefi),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for PowerMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

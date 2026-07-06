use crate::PowerAction;

/// Returns the Nerd Font icon name for a given power action.
pub fn power_action_icon(action: &PowerAction) -> &'static str {
    match action {
        PowerAction::Shutdown => "nf-md-power",
        PowerAction::Reboot => "nf-md-restart",
        PowerAction::Suspend => "nf-md-sleep",
        PowerAction::Hibernate => "nf-md-snowflake",
        PowerAction::Lock => "nf-md-lock",
        PowerAction::Logout => "nf-md-logout",
        PowerAction::RebootToFirmware => "nf-md-chip",
        PowerAction::Cancel => "nf-md-close",
    }
}

/// Returns the Nerd Font Unicode codepoint for a given power action.
pub fn power_action_icon_unicode(action: &PowerAction) -> &'static str {
    match action {
        PowerAction::Shutdown => "\u{f0425}",
        PowerAction::Reboot => "\u{f0450}",
        PowerAction::Suspend => "\u{f0913}",
        PowerAction::Hibernate => "\u{f0721}",
        PowerAction::Lock => "\u{f033e}",
        PowerAction::Logout => "\u{f0343}",
        PowerAction::RebootToFirmware => "\u{f0629}",
        PowerAction::Cancel => "\u{f0156}",
    }
}

use hyprland::ctl::notify::Icon;
use smearor_hyprland_model::HyprlandNotifyIcon;

pub(crate) fn convert_notify_icon(icon: HyprlandNotifyIcon) -> Icon {
    match icon {
        HyprlandNotifyIcon::Warning => Icon::Warning,
        HyprlandNotifyIcon::Info => Icon::Info,
        HyprlandNotifyIcon::Hint => Icon::Hint,
        HyprlandNotifyIcon::Error => Icon::Error,
        HyprlandNotifyIcon::Confused => Icon::Confused,
        HyprlandNotifyIcon::Ok => Icon::Ok,
        HyprlandNotifyIcon::NoIcon => Icon::NoIcon,
    }
}

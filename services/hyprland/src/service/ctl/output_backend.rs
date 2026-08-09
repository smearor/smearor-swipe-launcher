use hyprland::ctl::output::OutputBackends;
use smearor_hyprland_model::HyprlandOutputBackend;

pub(crate) fn convert_output_backend(backend: HyprlandOutputBackend) -> OutputBackends {
    match backend {
        HyprlandOutputBackend::Wayland => OutputBackends::Wayland,
        HyprlandOutputBackend::X11 => OutputBackends::X11,
        HyprlandOutputBackend::Headless => OutputBackends::Headless,
        HyprlandOutputBackend::Auto => OutputBackends::Auto,
    }
}

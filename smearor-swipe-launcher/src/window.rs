use crate::config::LauncherConfig;
use crate::display::DEFAULT_HEIGHT;
use crate::display::calculate_area_size;
use gtk4::ApplicationWindow;

pub fn create_window(app: &gtk4::Application, config: &LauncherConfig) -> ApplicationWindow {
    let rotation = config.launcher.rotation;
    let area_size = calculate_area_size(rotation, DEFAULT_HEIGHT);
    ApplicationWindow::builder()
        .application(app)
        .title("Smearor Swipe Launcher")
        .default_width(area_size.width)
        .default_height(area_size.height)
        .build()
}

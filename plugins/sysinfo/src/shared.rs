use crate::config::BarOrientation;
use crate::config::DisplayMode;
use crate::config::PercentageWidgetConfig;
use gtk4::Box as GtkBox;
use gtk4::DrawingArea;
use gtk4::Image;
use gtk4::Label;
use gtk4::LevelBar;
use gtk4::Orientation;
use gtk4::gio;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;

/// RGB color with components in the range [0.0, 1.0].
pub struct Color(pub f64, pub f64, pub f64);

/// Malachite (green).
pub const COLOR_MALACHITE: Color = Color(0.016, 0.906, 0.384);

/// Selective yellow (amber).
pub const COLOR_SELECTIVE_YELLOW: Color = Color(0.961, 0.718, 0.0);

/// Celestial blue.
pub const COLOR_CELESTIAL_BLUE: Color = Color(0.0, 0.631, 0.894);

/// Mexican pink.
pub const COLOR_MEXICAN_PINK: Color = Color(0.863, 0.0, 0.451);

/// Chartreuse (lime green).
pub const COLOR_CHARTREUSE: Color = Color(0.537, 0.988, 0.0);

/// Container for the UI elements of a percentage-based widget.
pub struct PercentageWidgetContainer {
    /// Outer container box.
    pub container: GtkBox,
    /// Optional icon image.
    pub icon_image: Option<Image>,
    /// Optional value label.
    pub value_label: Option<Label>,
    /// Optional bar widget.
    pub bar: Option<LevelBar>,
    /// Optional gauge widget.
    pub gauge: Option<DrawingArea>,
}

/// Builds the GTK container for a percentage-based widget according to its configuration.
pub fn build_percentage_widget(config: &PercentageWidgetConfig) -> PercentageWidgetContainer {
    let orientation = match config.bar_orientation {
        BarOrientation::Horizontal => Orientation::Horizontal,
        BarOrientation::Vertical => Orientation::Vertical,
    };
    let container = GtkBox::builder().orientation(Orientation::Horizontal).spacing(4).build();

    let mut icon_image = None;
    if config.show_icon {
        if let Some(ref icon) = config.icon {
            let image = build_icon_image(icon, config.icon_size);
            image.add_css_class("sysinfo-icon");
            container.append(&image);
            icon_image = Some(image);
        }
    }

    let mut value_label = None;
    if config.show_value {
        let label = Label::builder().css_classes(["sysinfo-value".to_string()]).build();
        container.append(&label);
        value_label = Some(label);
    }

    let mut bar = None;
    let mut gauge = None;

    match config.display_mode {
        DisplayMode::Bar => {
            let level_bar = LevelBar::builder()
                .orientation(orientation)
                .min_value(0.0)
                .max_value(100.0)
                .width_request(config.width)
                .height_request(config.height)
                .css_classes(["sysinfo-bar".to_string(), "sysinfo-normal".to_string()])
                .build();
            container.append(&level_bar);
            bar = Some(level_bar);
        }
        DisplayMode::Gauge => {
            let drawing_area = DrawingArea::builder()
                .content_width(config.width)
                .content_height(config.height)
                .css_classes(["sysinfo-gauge".to_string()])
                .build();
            container.append(&drawing_area);
            gauge = Some(drawing_area);
        }
    }

    PercentageWidgetContainer {
        container,
        icon_image,
        value_label,
        bar,
        gauge,
    }
}

/// Builds an icon image, resolving Nerd Fonts CSS class names to GTK icon names.
pub fn build_icon_image(icon_name: &str, size: i32) -> Image {
    let image = if icon_name.starts_with("nf-") {
        if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
            let resource_path = format!("/com/nerd/icons/{}.svg", gtk_icon_name);
            if gio::resources_lookup_data(&resource_path, gio::ResourceLookupFlags::NONE).is_ok() {
                Image::from_resource(&resource_path)
            } else {
                Image::from_icon_name(&gtk_icon_name)
            }
        } else {
            Image::from_icon_name(icon_name)
        }
    } else {
        Image::from_icon_name(icon_name)
    };
    image.set_pixel_size(size);
    image
}

/// Updates the value label using a simple placeholder replacement.
pub fn update_value_label(label: &Label, format: &str, value: f32, extra: Option<(char, f32)>) {
    let mut text = format.replace("{value:.0}", &format!("{:.0}", value));
    text = text.replace("{value:.1}", &format!("{:.1}", value));
    text = text.replace("{value:.2}", &format!("{:.2}", value));
    text = text.replace("{value}", &format!("{:.1}", value));
    if let Some((placeholder, extra_value)) = extra {
        text = text.replace(&format!("{{{:.0}}}", placeholder), &format!("{:.0}", extra_value));
        text = text.replace(&format!("{{{:.1}}}", placeholder), &format!("{:.1}", extra_value));
    }
    label.set_text(&text);
}

/// Determines the CSS class for a value based on thresholds.
pub fn value_class(value: f32, warning: f32, critical: f32) -> &'static str {
    if value >= critical {
        "sysinfo-critical"
    } else if value >= warning {
        "sysinfo-warning"
    } else {
        "sysinfo-normal"
    }
}

/// Selects the color for a gauge based on value and thresholds.
pub fn gauge_color(value: f32, warning: f32, critical: f32) -> Color {
    if value >= critical {
        COLOR_MEXICAN_PINK
    } else if value >= warning {
        COLOR_SELECTIVE_YELLOW
    } else {
        COLOR_MALACHITE
    }
}

/// Draws a semicircular gauge with the given value and color.
pub fn draw_gauge(context: &gtk4::cairo::Context, width: i32, height: i32, value: f32, color: Color) {
    let clamped_value = value.clamp(0.0, 100.0) / 100.0;
    let center_x = width as f64 / 2.0;
    let center_y = height as f64 / 2.0;
    let radius = (center_x.min(center_y) * 0.8).max(1.0);
    let start_angle = 0.75 * 2.0 * std::f64::consts::PI;
    let end_angle = start_angle + clamped_value as f64 * 1.5 * std::f64::consts::PI;

    context.set_source_rgb(0.2, 0.2, 0.2);
    let _ = context.arc(center_x, center_y, radius, 0.75 * 2.0 * std::f64::consts::PI, 0.25 * 2.0 * std::f64::consts::PI);
    let _ = context.stroke();

    context.set_source_rgb(color.0, color.1, color.2);
    let _ = context.arc(center_x, center_y, radius, start_angle, end_angle);
    let _ = context.stroke();
}

/// Formats bytes as a human-readable string.
pub fn format_bytes(bytes: u64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit_index = 0;
    while value >= 1024.0 && unit_index < units.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }
    format!("{:.1} {}", value, units[unit_index])
}

/// Formats seconds as a human-readable duration.
pub fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if days > 0 {
        format!("{}d {:02}h {:02}m {:02}s", days, hours, minutes, secs)
    } else {
        format!("{:02}h {:02}m {:02}s", hours, minutes, secs)
    }
}

use crate::config::BarOrientation;
use crate::config::DisplayMode;
use crate::config::PercentageWidgetConfig;
use glib::prelude::Cast;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::DrawingArea;
use gtk4::Image;
use gtk4::Label;
use gtk4::LevelBar;
use gtk4::Orientation;
use gtk4::Overlay;
use gtk4::Widget;
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
    /// Content box holding icon and value label (used for appending extra labels).
    pub container: GtkBox,
    /// Outer widget to return from `build_widget` (box for bar mode, overlay for gauge mode).
    pub outer_widget: Widget,
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
    let container_orientation = match config.display_mode {
        DisplayMode::Bar => Orientation::Horizontal,
        DisplayMode::Gauge => Orientation::Vertical,
    };
    let container = GtkBox::builder().orientation(container_orientation).spacing(4).build();

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
        label.set_halign(Align::Center);
        container.append(&label);
        value_label = Some(label);
    }

    let mut bar = None;
    let mut gauge = None;
    let outer_widget: Widget;

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
            outer_widget = container.clone().upcast::<Widget>();
        }
        DisplayMode::Gauge => {
            let size = config.width.max(config.height);
            let drawing_area = DrawingArea::builder()
                .content_width(size)
                .content_height(size)
                .css_classes(["sysinfo-gauge".to_string()])
                .build();

            let overlay = Overlay::builder().build();
            overlay.set_child(Some(&drawing_area));
            container.set_halign(Align::Center);
            container.set_valign(Align::Center);
            overlay.add_overlay(&container);
            overlay.set_margin_start(8);
            overlay.set_margin_end(8);

            gauge = Some(drawing_area);
            outer_widget = overlay.upcast::<Widget>();
        }
    }

    PercentageWidgetContainer {
        container,
        outer_widget,
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
///
/// `value` replaces the generic `{value}` placeholders. If `placeholder_name` is
/// non-empty, placeholders using that name are also replaced so existing configs
/// such as `{cpu_usage:.0}%` continue to work.
pub fn update_value_label(label: &Label, format: &str, value: f32, placeholder_name: &str) {
    let mut text = format.replace("{value:.0}", &format!("{:.0}", value));
    text = text.replace("{value:.1}", &format!("{:.1}", value));
    text = text.replace("{value:.2}", &format!("{:.2}", value));
    text = text.replace("{value}", &format!("{:.1}", value));

    if !placeholder_name.is_empty() {
        text = text.replace(&format!("{{{placeholder_name}:.0}}"), &format!("{:.0}", value));
        text = text.replace(&format!("{{{placeholder_name}:.1}}"), &format!("{:.1}", value));
        text = text.replace(&format!("{{{placeholder_name}:.2}}"), &format!("{:.2}", value));
        text = text.replace(&format!("{{{placeholder_name}}}"), &format!("{:.1}", value));
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

/// Container returned by `build_gauge_container` for custom gauge widgets.
pub struct GaugeContainer {
    /// Outer overlay widget.
    pub outer_widget: Widget,
    /// Vertical content box for icon and labels.
    pub content_box: GtkBox,
    /// Drawing area for the gauge.
    pub drawing_area: DrawingArea,
}

/// Builds a circular gauge overlay with a vertical content box centered on top.
///
/// This is used by widgets that need a custom gauge drawing (e.g. network) and
/// cannot reuse the single-value `build_percentage_widget` helper.
pub fn build_gauge_container(size: i32, css_class: &str) -> GaugeContainer {
    let drawing_area = DrawingArea::builder()
        .content_width(size)
        .content_height(size)
        .css_classes([css_class.to_string()])
        .build();

    let content_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .halign(Align::Center)
        .valign(Align::Center)
        .build();

    let overlay = Overlay::builder().build();
    overlay.set_child(Some(&drawing_area));
    overlay.add_overlay(&content_box);
    overlay.set_margin_start(8);
    overlay.set_margin_end(8);

    GaugeContainer {
        outer_widget: overlay.upcast::<Widget>(),
        content_box,
        drawing_area,
    }
}

/// Draws a circular gauge around the given value and color.
///
/// The gauge is drawn as a full ring with a thicker stroke so it can surround
/// the widget's icon and text.
pub fn draw_gauge(context: &gtk4::cairo::Context, width: i32, height: i32, value: f32, color: Color) {
    let clamped_value = value.clamp(0.0, 100.0) / 100.0;
    let center_x = width as f64 / 2.0;
    let center_y = height as f64 / 2.0;
    let line_width = 10.0;
    let radius = (center_x.min(center_y) - line_width / 2.0 - 2.0).max(1.0);
    let start_angle = 1.5 * std::f64::consts::PI;
    let end_angle = start_angle + clamped_value as f64 * 2.0 * std::f64::consts::PI;

    context.set_line_width(line_width);

    context.set_source_rgb(0.2, 0.2, 0.2);
    let _ = context.arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI);
    let _ = context.stroke();

    context.set_source_rgb(color.0, color.1, color.2);
    let _ = context.arc(center_x, center_y, radius, start_angle, end_angle);
    let _ = context.stroke();
}

/// Draws a network gauge with two semicircles: left for download, right for upload.
pub fn draw_network_gauge(
    context: &gtk4::cairo::Context,
    width: i32,
    height: i32,
    download_value: f32,
    upload_value: f32,
    download_color: Color,
    upload_color: Color,
) {
    let clamped_download = download_value.clamp(0.0, 100.0) / 100.0;
    let clamped_upload = upload_value.clamp(0.0, 100.0) / 100.0;
    let center_x = width as f64 / 2.0;
    let center_y = height as f64 / 2.0;
    let line_width = 10.0;
    let radius = (center_x.min(center_y) - line_width / 2.0 - 2.0).max(1.0);

    context.set_line_width(line_width);

    context.set_source_rgb(0.2, 0.2, 0.2);
    let _ = context.arc(center_x, center_y, radius, 0.5 * std::f64::consts::PI, 1.5 * std::f64::consts::PI);
    let _ = context.stroke();
    let _ = context.arc(center_x, center_y, radius, 1.5 * std::f64::consts::PI, 2.5 * std::f64::consts::PI);
    let _ = context.stroke();

    context.set_source_rgb(download_color.0, download_color.1, download_color.2);
    let download_end = 0.5 * std::f64::consts::PI + clamped_download as f64 * std::f64::consts::PI;
    let _ = context.arc(center_x, center_y, radius, 0.5 * std::f64::consts::PI, download_end);
    let _ = context.stroke();

    context.set_source_rgb(upload_color.0, upload_color.1, upload_color.2);
    let upload_end = 1.5 * std::f64::consts::PI + clamped_upload as f64 * std::f64::consts::PI;
    let _ = context.arc(center_x, center_y, radius, 1.5 * std::f64::consts::PI, upload_end);
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

/// Formats seconds using a custom template.
///
/// Supported placeholders: `{days}`, `{hours}`, `{minutes}`, `{seconds}` and `{value}`
/// (the default human-readable duration).
pub fn format_duration_with_format(seconds: u64, format: &str) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut text = format.to_string();
    text = text.replace("{days}", &days.to_string());
    text = text.replace("{hours}", &format!("{:02}", hours));
    text = text.replace("{minutes}", &format!("{:02}", minutes));
    text = text.replace("{seconds}", &format!("{:02}", secs));
    text = text.replace("{value}", &format_duration(seconds));
    text
}

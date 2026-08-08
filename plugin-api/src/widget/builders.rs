//! Reusable GTK builder helpers for widget construction.
//!
//! Provides factory functions to eliminate duplicated `build_widget`
//! boilerplate across plugins.

use gtk4::Align;
use gtk4::Box;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::pango::EllipsizeMode;
use gtk4::prelude::WidgetExt;

use crate::nerd_font::apply_icon_color;
use crate::nerd_font::apply_text_color;
use crate::widget::Color;

/// Builds a vertical content box (centered, vexpand) with the given spacing and CSS classes.
///
/// Used as the main container inside widget buttons.
pub fn build_content_box(spacing: i32, css_classes: &[&str]) -> Box {
    Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(spacing)
        .valign(Align::Center)
        .halign(Align::Center)
        .vexpand(true)
        .css_classes(css_classes)
        .build()
}

/// Builds a vertical info box (left-aligned, centered) for wide-mode sub-containers.
pub fn build_info_box(spacing: i32) -> Box {
    Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(spacing)
        .valign(Align::Center)
        .halign(Align::Start)
        .build()
}

/// Builds a main text label (`widget-main-text`) with height 20 and optional text color.
///
/// When `ellipsize` is true, sets `EllipsizeMode::End`.
/// When `max_width_chars` is `Some`, sets the label's `max_width_chars` property.
pub fn build_main_label(text: &str, text_color: Option<Color>, ellipsize: bool, max_width_chars: Option<i32>) -> Label {
    let mut builder = Label::builder().label(text).css_classes(["widget-main-text"]);
    if ellipsize {
        builder = builder.ellipsize(EllipsizeMode::End);
    }
    if let Some(chars) = max_width_chars {
        builder = builder.max_width_chars(chars);
    }
    let label = builder.build();
    label.set_height_request(20);
    apply_text_color(&label, text_color);
    label
}

/// Builds an info text label (`widget-info-text`) with height 16 and optional text color.
///
/// When `ellipsize` is true, sets `EllipsizeMode::End`.
/// When `max_width_chars` is `Some`, sets the label's `max_width_chars` property.
pub fn build_info_label(text: &str, text_color: Option<Color>, ellipsize: bool, max_width_chars: Option<i32>) -> Label {
    let mut builder = Label::builder().label(text).css_classes(["widget-info-text"]);
    if ellipsize {
        builder = builder.ellipsize(EllipsizeMode::End);
    }
    if let Some(chars) = max_width_chars {
        builder = builder.max_width_chars(chars);
    }
    let label = builder.build();
    label.set_height_request(16);
    apply_text_color(&label, text_color);
    label
}

/// Builds a spacer label with the given height request.
pub fn build_spacer(height: i32) -> Label {
    let spacer = Label::new(Some(""));
    spacer.set_height_request(height);
    spacer
}

/// Builds a widget icon with `nerd-icon` CSS class, pixel size, and optional color.
///
/// The `setup_fn` closure is called after the icon is created to set the initial
/// icon name or codepoint (e.g. `Self::set_audio_icon(&icon, ...)`).
pub fn build_widget_icon(icon_size: i32, icon_color: Option<Color>, setup_fn: impl FnOnce(&Image)) -> Image {
    let icon = Image::new();
    icon.set_pixel_size(icon_size);
    icon.add_css_class("nerd-icon");
    setup_fn(&icon);
    if let Some(color) = icon_color {
        apply_icon_color(&icon, color);
    }
    icon
}

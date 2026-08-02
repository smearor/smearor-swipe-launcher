//! GTK widget construction and label updates for atomic widgets.

use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureLongPress;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::PropagationPhase;
use gtk4::Widget;
use gtk4::prelude::BoxExt;
use gtk4::prelude::Cast;
use gtk4::prelude::GestureExt;
use gtk4::prelude::GestureSingleExt;
use gtk4::prelude::WidgetExt;

use crate::MessageBroadcasterInner;
use crate::atomic::config::AtomicWidgetConfig;

/// Parameters for building the GTK widget layout of an atomic widget.
pub struct AtomicWidgetBuildParams {
    /// CSS class prefix for the outer box and labels (e.g. "audio", "mpris", "weather").
    pub css_prefix: &'static str,
    /// Default icon character (Nerd Font codepoint) shown while loading.
    pub default_icon: char,
    /// Default main label text shown while loading.
    pub default_main: &'static str,
    /// Default info label text shown while loading.
    pub default_info: &'static str,
}

/// Builds the GTK widget layout for an atomic widget.
///
/// Creates a vertical `GtkBox` with three labels (icon, main, info) and attaches
/// click and long-press gesture handlers that broadcast to the configured topics.
///
/// Returns the outer box widget and the three labels for later updates.
pub fn build_atomic_widget(
    broadcaster: &MessageBroadcasterInner,
    config: &AtomicWidgetConfig,
    params: &AtomicWidgetBuildParams,
) -> (Widget, Label, Label, Label) {
    let outer_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .css_classes([format!("{}-widget", params.css_prefix)])
        .halign(Align::Center)
        .valign(Align::Center)
        .build();

    let icon_label = Label::builder().css_classes([format!("{}-icon", params.css_prefix)]).build();
    let main_label = Label::builder().css_classes(["widget-main-text".to_string()]).build();
    let info_label = Label::builder().css_classes(["widget-info-text".to_string()]).build();

    icon_label.set_text(&params.default_icon.to_string());
    main_label.set_text(params.default_main);
    info_label.set_text(params.default_info);

    main_label.set_height_request(20);
    info_label.set_height_request(16);

    outer_box.append(&icon_label);
    outer_box.append(&main_label);
    outer_box.append(&info_label);

    let click_binding = config.click.as_binding();
    let longpress_binding = config.longpress.as_binding();
    let message_broadcaster = broadcaster.clone();

    let click_gesture = GestureClick::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
    let broadcaster_for_click = message_broadcaster.clone();
    click_gesture.connect_released(move |gesture, _n_press, _x, _y| {
        if let Some(seq) = gesture.current_sequence() {
            let state = gesture.sequence_state(&seq);
            if state == EventSequenceState::Claimed || state == EventSequenceState::Denied {
                return;
            }
        }
        click_binding.dispatch(&broadcaster_for_click);
        gesture.set_state(EventSequenceState::Claimed);
    });
    outer_box.add_controller(click_gesture);

    let longpress_gesture = GestureLongPress::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
    let broadcaster_for_longpress = message_broadcaster.clone();
    longpress_gesture.connect_pressed(move |gesture, _x, _y| {
        longpress_binding.dispatch(&broadcaster_for_longpress);
        gesture.set_state(EventSequenceState::Claimed);
    });
    outer_box.add_controller(longpress_gesture);

    (outer_box.upcast::<Widget>(), icon_label, main_label, info_label)
}

/// Updates three GTK labels with the given icon, main, and info text.
pub fn update_labels(icon_label: &Option<Label>, main_label: &Option<Label>, info_label: &Option<Label>, icon: &str, main: &str, info: &str) {
    if let Some(label) = icon_label {
        label.set_text(icon);
    }
    if let Some(label) = main_label {
        label.set_text(main);
    }
    if let Some(label) = info_label {
        label.set_text(info);
    }
}

use crate::widget::AppLauncherWidget;
use abi_stable::RRef;
use abi_stable::std_types::ROption;
use abi_stable::std_types::RResult;
use abi_stable::std_types::RString;
use gtk4::Align;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureLongPress;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::PropagationPhase;
use gtk4::Widget;
use gtk4::ffi::GtkWidget;
use gtk4::glib::translate::ToGlibPtr;
use gtk4::prelude::*;
use smearor_app_launcher_model::DesktopFileCommandAction;
use smearor_app_launcher_model::DesktopFileCommandMessage;
use smearor_app_launcher_model::TOPIC_COMMAND;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiWidget;
use smearor_swipe_launcher_plugin_api::LoadedPlugin;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginVTable;
use tracing::Level;
use tracing::debug;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::FmtSubscriber;

pub mod config;
pub mod desktop_entry;
pub mod widget;

unsafe extern "C" fn destroy_widget(plugin: *mut ()) {
    if !plugin.is_null() {
        unsafe {
            let _ = Box::from_raw(plugin as *mut AppLauncherWidget);
        }
    }
}

unsafe extern "C" fn get_id(plugin: *mut ()) -> RString {
    if plugin.is_null() {
        return RString::from("");
    }
    let widget = unsafe { &*(plugin as *const AppLauncherWidget) };
    widget.meta.id.clone()
}

unsafe extern "C" fn get_display_name(plugin: *mut ()) -> RString {
    if plugin.is_null() {
        return RString::from("");
    }
    let widget = unsafe { &*(plugin as *const AppLauncherWidget) };
    widget.meta.display_name.clone()
}

unsafe extern "C" fn get_icon_name(plugin: *mut ()) -> ROption<RString> {
    if plugin.is_null() {
        return ROption::RNone;
    }
    let widget = unsafe { &*(plugin as *const AppLauncherWidget) };
    widget.meta.icon_name.clone()
}

unsafe extern "C" fn build_widget(plugin: *mut ()) -> FfiWidget {
    if plugin.is_null() {
        return FfiWidget {
            raw_widget: std::ptr::null_mut(),
        };
    }

    let result = std::panic::catch_unwind(|| {
        let _ = adw::init();
        let widget = unsafe { &mut *(plugin as *mut AppLauncherWidget) };

        let main_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .width_request(100)
            .height_request(100)
            .halign(Align::Center)
            .valign(Align::Center)
            .css_classes(["app-launcher-tile"])
            .build();

        // Render Icon
        let image = gtk4::Image::from_icon_name(&widget.icon_name);
        image.set_pixel_size(48);
        main_box.append(&image);

        // Render Name
        let label = Label::builder()
            .label(&widget.app_name)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(12)
            .css_classes(["app-launcher-label"])
            .build();
        main_box.append(&label);

        // LED Indicator Box to show if application is running
        let led_box = gtk4::Box::builder()
            .width_request(8)
            .height_request(8)
            .halign(Align::Center)
            .css_classes(["app-launcher-led", "led-unlit"])
            .build();
        main_box.append(&led_box);

        *widget.led_indicator.write().unwrap() = Some(led_box);

        // Gestures - Click to Launch
        let longpress_gesture = GestureLongPress::builder()
            .propagation_phase(PropagationPhase::Capture)
            // Extra long because of the parent scroll window widget has a drag gesture
            .delay_factor(2.0)
            .build();

        let click_gesture = GestureClick::builder().propagation_phase(PropagationPhase::Capture).build();
        longpress_gesture.group_with(&click_gesture);

        click_gesture.connect_pressed(move |_, _, _, _| {});

        let desktop_file_inner = widget.config.desktop_file_path.clone();
        let message_broadcaster = widget.get_broadcaster();
        click_gesture.connect_released(move |gesture, n_clicks, _, _| {
            if let Some(seq) = gesture.current_sequence() {
                let state = gesture.sequence_state(&seq);
                if state == EventSequenceState::Claimed || state == EventSequenceState::Denied {
                    return;
                }
            }
            info!("Click released {n_clicks}");
            message_broadcaster.broadcast_message(TOPIC_COMMAND, DesktopFileCommandMessage::exec(&desktop_file_inner));
            gesture.set_state(EventSequenceState::Claimed);
        });

        let main_box_inner = main_box.downgrade();
        longpress_gesture.connect_begin(move |_, _| {
            if let Some(main_box) = main_box_inner.upgrade() {
                main_box.add_css_class("longpress");
            }
        });
        let desktop_file_inner = widget.config.desktop_file_path.clone();
        let message_broadcaster = widget.get_broadcaster();
        longpress_gesture.connect_pressed(move |gesture, n_clicks, _| {
            message_broadcaster.broadcast_message(TOPIC_COMMAND, DesktopFileCommandMessage::terminate(&desktop_file_inner));
            gesture.set_state(EventSequenceState::Claimed);
        });

        let main_box_inner = main_box.downgrade();
        longpress_gesture.connect_end(move |gesture, _| {
            if let Some(main_box) = main_box_inner.upgrade() {
                main_box.remove_css_class("longpress");
            }
        });
        longpress_gesture.connect_cancelled(move |gesture| {
            gesture.set_state(EventSequenceState::None);
        });

        main_box.add_controller(click_gesture);
        main_box.add_controller(longpress_gesture);

        let widget_obj = main_box.upcast::<Widget>();
        let stable_pointer: *mut GtkWidget = widget_obj.to_glib_full();
        FfiWidget { raw_widget: stable_pointer }
    });

    result.unwrap_or(FfiWidget {
        raw_widget: std::ptr::null_mut(),
    })
}

unsafe extern "C" fn on_message(plugin: *mut (), message: FfiEnvelope) {
    if plugin.is_null() {
        return;
    }
    // if message.topic != TOPIC_STATUS {
    //     return;
    // }
    let widget = unsafe { &*(plugin as *const AppLauncherWidget) };
    widget.handle_envelope_message(message);
}

unsafe extern "C" fn on_primary_action(_plugin: *mut (), _rotation: u32) -> i32 {
    0
}

unsafe extern "C" fn on_secondary_action(_plugin: *mut (), _rotation: u32) -> i32 {
    0
}

static VTABLE: PluginVTable = PluginVTable {
    destroy: destroy_widget,
    get_id,
    get_display_name,
    get_icon_name,
    build_widget,
    on_message,
    on_primary_action,
    on_secondary_action,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn smearor_plugin_create(
    config_json: *const i8,
    config_len: usize,
    core_context: FfiCoreContext,
) -> RResult<LoadedPlugin, PluginConstructionError> {
    if config_json.is_null() {
        return RResult::RErr(PluginConstructionError::ConfigJsonIsNull);
    }

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::DEBUG.into()))
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);

    let slice = unsafe { std::slice::from_raw_parts(config_json as *const u8, config_len) };
    let config_str = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(e) => return RResult::RErr(PluginConstructionError::InvalidUtf8Config(e.to_string().into())),
    };
    debug!("AppLauncherPlugin plugin_create: {config_str}");

    let config_value: serde_json::Value = match serde_json::from_str(config_str) {
        Ok(v) => v,
        Err(e) => return RResult::RErr(PluginConstructionError::FailedToParseConfig(e.to_string().into())),
    };

    let config = PluginConfig { config: config_value };
    let core_context = if core_context.core_obj.is_null() { None } else { Some(core_context) };

    match AppLauncherWidget::new(config, core_context) {
        Ok(widget) => {
            let widget_box = Box::new(widget);
            let plugin_instance = Box::into_raw(widget_box) as *mut ();
            RResult::ROk(LoadedPlugin {
                plugin_instance,
                vtable: RRef::new(&VTABLE),
            })
        }
        Err(e) => RResult::RErr(e),
    }
}

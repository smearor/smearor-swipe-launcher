use abi_stable::RRef;
use abi_stable::std_types::ROption;
use abi_stable::std_types::RResult;
use abi_stable::std_types::RString;
use gtk4::Widget;
use gtk4::glib::translate::ToGlibPtr;
use gtk4::prelude::*;
use smearor_plugin_api::FfiCoreContext;
use smearor_plugin_api::FfiEnvelope;
use smearor_plugin_api::FfiWidget;
use smearor_plugin_api::LoadedPlugin;
use smearor_plugin_api::PluginConfig;
use smearor_plugin_api::PluginConstructionError;
use smearor_plugin_api::PluginMeta;
use smearor_plugin_api::PluginMetaRaw;
use smearor_plugin_api::PluginVTable;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;
use tracing::error;
use tracing::info;

struct DesktopEntry {
    name: String,
    icon: String,
}

impl DesktopEntry {
    fn parse(path: &str) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        let mut name = None;
        let mut icon = None;
        let mut in_desktop_entry = false;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('[') && line.ends_with(']') {
                in_desktop_entry = line == "[Desktop Entry]";
                continue;
            }
            if !in_desktop_entry {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                match key.trim() {
                    "Name" => name = Some(val.trim().to_string()),
                    "Icon" => icon = Some(val.trim().to_string()),
                    _ => {}
                }
            }
        }

        Some(DesktopEntry {
            name: name.unwrap_or_else(|| "Unknown App".to_string()),
            icon: icon.unwrap_or_else(|| "system-run".to_string()),
        })
    }
}

pub struct AppLauncherWidget {
    pub meta: PluginMeta,
    pub desktop_file_path: String,
    pub app_name: String,
    pub icon_name: String,
    pub core_context: Option<FfiCoreContext>,
    pub led_indicator: Arc<RwLock<Option<gtk4::Box>>>,
}

impl AppLauncherWidget {
    pub fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionError> {
        let meta_raw: PluginMetaRaw =
            serde_json::from_value(config.config.clone()).map_err(|e| PluginConstructionError::FailedToParseMetaData(e.to_string().into()))?;

        // Extract desktop file path from config
        let desktop_file_path = config
            .config
            .get("desktop_file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PluginConstructionError::FailedToParseWidgetConfig("Missing 'desktop_file_path' in config".into()))?
            .to_string();

        let mut app_name = meta_raw.display_name.to_string();
        let mut icon_name = meta_raw.icon_name.unwrap_or_default().to_string();

        // Parse `.desktop` file
        if let Some(entry) = DesktopEntry::parse(&desktop_file_path) {
            app_name = entry.name;
            icon_name = entry.icon;
        } else {
            error!("Could not load .desktop file at: {}", desktop_file_path);
        }

        if icon_name.is_empty() {
            icon_name = "system-run".to_string(); // fallback
        }

        Ok(AppLauncherWidget {
            meta: PluginMeta::new(meta_raw.id, app_name.clone(), Some(icon_name.clone())),
            desktop_file_path,
            app_name,
            icon_name,
            core_context,
            led_indicator: Arc::new(RwLock::new(None)),
        })
    }
}

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
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .width_request(100)
            .height_request(100)
            .css_classes(["app-launcher-tile"])
            .build();

        // Render Icon
        let image = gtk4::Image::from_icon_name(&widget.icon_name);
        image.set_pixel_size(48);
        main_box.append(&image);

        // Render Name
        let label = gtk4::Label::builder()
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
            .halign(gtk4::Align::Center)
            .css_classes(["app-launcher-led", "led-unlit"])
            .build();
        main_box.append(&led_box);

        *widget.led_indicator.write().unwrap() = Some(led_box);

        // Gestures - Click to Launch
        let click_gesture = gtk4::GestureClick::new();
        let core_context_clone1 = widget.core_context.clone();
        let desktop_file_clone1 = widget.desktop_file_path.clone();
        let widget_id_clone1 = widget.meta.id.clone();
        click_gesture.connect_pressed(move |_, _, _, _| {
            info!("AppLauncher Widget: Single-click/tap detected for {}", desktop_file_clone1);
            if let Some(ref context) = core_context_clone1 {
                let envelope = FfiEnvelope {
                    sender_id: widget_id_clone1.clone(),
                    topic: RString::from("service/app_launcher/command"),
                    payload: RString::from(format!("{{\"action\": \"Launch\", \"desktop_file\": \"{}\"}}", desktop_file_clone1)),
                };
                unsafe {
                    (context.vtable.get().send_message)(context.core_obj, envelope);
                }
            }
        });
        main_box.add_controller(click_gesture);

        // Gestures - Longpress to Terminate
        let longpress_gesture = gtk4::GestureLongPress::new();
        let core_context_clone2 = widget.core_context.clone();
        let desktop_file_clone2 = widget.desktop_file_path.clone();
        let widget_id_clone2 = widget.meta.id.clone();
        longpress_gesture.connect_pressed(move |_, _, _| {
            info!("AppLauncher Widget: Longpress detected for {}", desktop_file_clone2);
            if let Some(ref context) = core_context_clone2 {
                let envelope = FfiEnvelope {
                    sender_id: widget_id_clone2.clone(),
                    topic: RString::from("service/app_launcher/command"),
                    payload: RString::from(format!("{{\"action\": \"Terminate\", \"desktop_file\": \"{}\"}}", desktop_file_clone2)),
                };
                unsafe {
                    (context.vtable.get().send_message)(context.core_obj, envelope);
                }
            }
        });
        main_box.add_controller(longpress_gesture);

        let widget_obj = main_box.upcast::<Widget>();
        let stable_pointer: *mut gtk4::ffi::GtkWidget = widget_obj.to_glib_full();
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

    let widget = unsafe { &*(plugin as *const AppLauncherWidget) };
    let topic = message.topic.to_string();
    let payload = message.payload.to_string();

    debug!("AppLauncher Widget {} received message on '{}'", widget.meta.id, topic);

    if topic == "service/app_launcher/status" {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&payload) {
            let desktop_file = parsed.get("desktop_file").and_then(|v| v.as_str()).unwrap_or_default();
            if desktop_file == widget.desktop_file_path {
                let status = parsed.get("status").and_then(|v| v.as_str()).unwrap_or_default();
                info!("AppLauncher Widget {} status updated for {}: {}", widget.meta.id, desktop_file, status);
                if let Ok(guard) = widget.led_indicator.read() {
                    if let Some(led) = guard.as_ref() {
                        if status == "Running" {
                            led.remove_css_class("led-unlit");
                            led.add_css_class("led-lit");
                        } else {
                            led.remove_css_class("led-lit");
                            led.add_css_class("led-unlit");
                        }
                    }
                }
            }
        }
    }
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

    let slice = unsafe { std::slice::from_raw_parts(config_json as *const u8, config_len) };
    let config_str = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(e) => return RResult::RErr(PluginConstructionError::InvalidUtf8Config(e.to_string().into())),
    };

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

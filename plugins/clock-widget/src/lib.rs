pub mod clock;
pub mod config;
pub mod widget;

use crate::widget::ClockWidget;
use abi_stable::RRef;
use abi_stable::std_types::ROption;
use abi_stable::std_types::RResult;
use abi_stable::std_types::RString;
use adw::StatusPage;
use gtk4::Widget;
use gtk4::glib::translate::ToGlibPtr;
use gtk4::prelude::*;
use smearor_plugin_api::FfiCoreContext;
use smearor_plugin_api::FfiWidget;
use smearor_plugin_api::LoadedPlugin;
use smearor_plugin_api::PluginConfig;
use smearor_plugin_api::PluginConstructionError;
use smearor_plugin_api::PluginVTable;

unsafe extern "C" fn destroy_clock_widget(plugin: *mut ()) {
    if !plugin.is_null() {
        unsafe {
            let _ = Box::from_raw(plugin as *mut ClockWidget);
        }
    }
}

unsafe extern "C" fn get_id(plugin: *mut ()) -> RString {
    if plugin.is_null() {
        return RString::from("");
    }

    let widget = unsafe { &*(plugin as *const ClockWidget) };
    widget.meta.id.clone()
}

unsafe extern "C" fn get_display_name(plugin: *mut ()) -> RString {
    if plugin.is_null() {
        return RString::from("");
    }

    let widget = unsafe { &*(plugin as *const ClockWidget) };
    widget.meta.display_name.clone()
}

unsafe extern "C" fn get_icon_name(plugin: *mut ()) -> ROption<RString> {
    if plugin.is_null() {
        return ROption::RNone;
    }

    let widget = unsafe { &*(plugin as *const ClockWidget) };
    widget.meta.icon_name.clone()
}

unsafe extern "C" fn build_widget(plugin: *mut ()) -> FfiWidget {
    if plugin.is_null() {
        return FfiWidget {
            raw_widget: std::ptr::null_mut(),
        };
    }

    let result = std::panic::catch_unwind(|| {
        let widget = unsafe { &mut *(plugin as *mut ClockWidget) };

        let _ = adw::init();

        let status_page = StatusPage::builder()
            .title(widget.clock.get_current_time())
            .description(widget.clock.config.description.clone().as_str())
            .width_request(200)
            .build();
        status_page.add_css_class("smart-desk-clock");

        *widget.status_page.write().unwrap() = Some(status_page.clone());
        if let Some(time_receiver) = widget.time_receiver.take() {
            widget.start_time_update(time_receiver);
        }

        let widget_obj = status_page.upcast::<Widget>();
        let stable_pointer: *mut gtk4::ffi::GtkWidget = widget_obj.to_glib_full();
        FfiWidget { raw_widget: stable_pointer }
    });

    result.unwrap_or(FfiWidget {
        raw_widget: std::ptr::null_mut(),
    })
}

unsafe extern "C" fn on_primary_action(plugin: *mut (), _rotation: u32) -> i32 {
    if plugin.is_null() {
        return -1;
    }

    let _widget = unsafe { (plugin as *const ClockWidget).as_ref() };
    0
}

unsafe extern "C" fn on_secondary_action(plugin: *mut (), _rotation: u32) -> i32 {
    if plugin.is_null() {
        return -1;
    }

    let _widget = unsafe { (plugin as *const ClockWidget).as_ref() };
    0
}

static VTABLE: PluginVTable = PluginVTable {
    destroy: destroy_clock_widget,
    get_id,
    get_display_name,
    get_icon_name,
    build_widget,
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

    let clock_widget = match ClockWidget::new(config, core_context) {
        Ok(clock_widget) => clock_widget,
        Err(e) => {
            return RResult::RErr(e);
        }
    };
    let widget = Box::new(clock_widget);
    let plugin_instance = Box::into_raw(widget) as *mut ();

    let loaded_plugin = LoadedPlugin {
        plugin_instance,
        vtable: RRef::new(&VTABLE),
    };

    RResult::ROk(loaded_plugin)
}

use crate::instance::LauncherInstance;
use smearor_swipe_launcher_plugin_api::FfiHtmlString;
use tracing::error;

/// Collected plugin data needed to call `render_html` without holding
/// a DashMap reference across a `catch_unwind` boundary.
struct PluginRenderInfo {
    plugin_id: String,
    instance_ptr: *mut core::ffi::c_void,
    render_html: unsafe extern "C" fn(
        instance: *mut core::ffi::c_void,
        instance_id: *const u8,
        instance_id_len: usize,
        plugin_id: *const u8,
        plugin_id_len: usize,
    ) -> FfiHtmlString,
}

/// Render HTML fragments for the plugins of the currently visible area.
pub fn render_all_widgets_html(instance: &LauncherInstance) -> String {
    let plugin_manager = &instance.plugin_manager;

    // Get the plugin entries of the currently visible area, in config order.
    let visible_entries = match instance.area_manager.lock() {
        Ok(area_manager) => area_manager.visible_area_plugin_entries(),
        Err(_) => return String::new(),
    };

    // Collect render info for only the visible area's plugins, in order.
    let instance_id = &instance.instance_id;
    let render_infos: Vec<PluginRenderInfo> = visible_entries
        .iter()
        .filter(|entry| !entry.disabled)
        .filter_map(|entry| {
            let namespaced_id = format!("{}:{}", instance_id, entry.id);
            let loaded = plugin_manager.plugins.get(&namespaced_id)?;
            let vtable = loaded.vtable;
            if vtable.is_null() || loaded.instance.is_null() {
                return None;
            }
            let render_html = unsafe { (*vtable).render_html }?;
            Some(PluginRenderInfo {
                plugin_id: namespaced_id,
                instance_ptr: loaded.instance,
                render_html,
            })
        })
        .collect();

    let instance_id_bytes = instance.instance_id.as_bytes();

    let mut fragments = Vec::new();
    for info in render_infos {
        let plugin_id_bytes = info.plugin_id.as_bytes();

        let result = std::panic::catch_unwind(|| {
            let ffi_string: FfiHtmlString = unsafe {
                (info.render_html)(
                    info.instance_ptr,
                    instance_id_bytes.as_ptr(),
                    instance_id_bytes.len(),
                    plugin_id_bytes.as_ptr(),
                    plugin_id_bytes.len(),
                )
            };
            ffi_string.as_str().to_string()
        });

        match result {
            Ok(html) => {
                fragments.push(html);
            }
            Err(_) => {
                error!("Plugin {} panicked during render_html", info.plugin_id);
            }
        }
    }

    fragments.join("\n")
}

/// Render HTML for a single widget by its namespaced plugin ID.
pub fn render_single_widget_html(instance: &LauncherInstance, namespaced_id: &str) -> String {
    let plugin_manager = &instance.plugin_manager;

    let (instance_ptr, render_html) = {
        let loaded = match plugin_manager.plugins.get(namespaced_id) {
            Some(p) => p,
            None => return String::new(),
        };
        let vtable = loaded.vtable;
        if vtable.is_null() || loaded.instance.is_null() {
            return String::new();
        }
        let render_html = unsafe { (*vtable).render_html };
        let Some(render_html) = render_html else {
            return String::new();
        };
        (loaded.instance, render_html)
    };

    let instance_id_bytes = instance.instance_id.as_bytes();
    let plugin_id_bytes = namespaced_id.as_bytes();

    let result = std::panic::catch_unwind(|| {
        let ffi_string: FfiHtmlString = unsafe {
            (render_html)(
                instance_ptr,
                instance_id_bytes.as_ptr(),
                instance_id_bytes.len(),
                plugin_id_bytes.as_ptr(),
                plugin_id_bytes.len(),
            )
        };
        ffi_string.as_str().to_string()
    });

    match result {
        Ok(html) => html,
        Err(_) => {
            error!("Plugin {} panicked during render_html", namespaced_id);
            String::new()
        }
    }
}

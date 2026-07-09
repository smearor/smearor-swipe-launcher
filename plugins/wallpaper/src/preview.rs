use gtk4::Picture;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use tracing::debug;

/// Loads a preview image from the given path into the `gtk4::Picture`.
/// Falls back to showing the icon name in the `gtk4::Image` if the image cannot be loaded.
/// The `preview_icon` should be a Nerd Font icon name (e.g. `nf-md-flower`).
pub fn update_preview(
    picture: &Rc<RefCell<Option<Picture>>>,
    fallback_image: &Rc<RefCell<Option<gtk4::Image>>>,
    preview_path: &str,
    preview_icon: &str,
    global_fallback_icon: &str,
) {
    if preview_path.is_empty() {
        set_fallback_icon(fallback_image, preview_icon, global_fallback_icon);
        return;
    }

    let path = Path::new(preview_path);
    if !path.exists() {
        debug!("wallpaper widget: preview image not found: {}", preview_path);
        set_fallback_icon(fallback_image, preview_icon, global_fallback_icon);
        return;
    }

    if let Some(ref pic) = *picture.borrow() {
        pic.set_filename(Some(path));
    }
}

/// Sets the fallback icon on the `gtk4::Image`.
/// Uses the per-theme `preview_icon` if non-empty, otherwise falls back to `global_fallback_icon`.
/// Resolves the Nerd Font name to a GResource SVG path and loads it via `Image::from_resource`.
fn set_fallback_icon(fallback_image: &Rc<RefCell<Option<gtk4::Image>>>, preview_icon: &str, global_fallback_icon: &str) {
    let icon_name = if preview_icon.is_empty() { global_fallback_icon } else { preview_icon };
    if let Some(ref img) = *fallback_image.borrow() {
        if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
            let resource_path = format!("/com/nerd/icons/{}.svg", gtk_icon_name);
            if gtk4::gio::resources_lookup_data(&resource_path, gtk4::gio::ResourceLookupFlags::NONE).is_ok() {
                img.set_resource(Some(&resource_path));
            } else {
                debug!("wallpaper widget: GResource not found for icon '{}': {}", icon_name, resource_path);
            }
        }
    }
}

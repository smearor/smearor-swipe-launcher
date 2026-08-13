use gtk4::Image;
use gtk4::gdk::Texture;
use gtk4::gio;
use gtk4::prelude::WidgetExt;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use tracing::debug;

/// Loads a preview image from the given path into the `gtk4::Image`.
/// Falls back to showing the icon name in the fallback `gtk4::Image` if the image cannot be loaded.
/// The `preview_icon` should be a Nerd Font icon name (e.g. `nf-md-palette`).
pub fn update_preview(
    preview_image: &Rc<RefCell<Option<Image>>>,
    fallback_image: &Rc<RefCell<Option<Image>>>,
    preview_path: &str,
    preview_icon: &str,
    global_fallback_icon: &str,
) {
    if preview_path.is_empty() {
        show_fallback_only(preview_image, fallback_image);
        set_fallback_icon(fallback_image, preview_icon, global_fallback_icon);
        return;
    }

    let path = Path::new(preview_path);
    if !path.exists() {
        debug!("theme widget: preview image not found: {}", preview_path);
        show_fallback_only(preview_image, fallback_image);
        set_fallback_icon(fallback_image, preview_icon, global_fallback_icon);
        return;
    }

    let file = gio::File::for_path(path);
    match Texture::from_file(&file) {
        Ok(texture) => {
            if let Some(ref img) = *preview_image.borrow() {
                img.set_paintable(Some(&texture));
                img.set_visible(true);
            }
            if let Some(ref img) = *fallback_image.borrow() {
                img.set_visible(false);
            }
        }
        Err(e) => {
            debug!("theme widget: failed to load preview image '{}': {}", preview_path, e);
            show_fallback_only(preview_image, fallback_image);
            set_fallback_icon(fallback_image, preview_icon, global_fallback_icon);
        }
    }
}

/// Hides the preview image and shows the fallback icon.
fn show_fallback_only(preview_image: &Rc<RefCell<Option<Image>>>, fallback_image: &Rc<RefCell<Option<Image>>>) {
    if let Some(ref img) = *preview_image.borrow() {
        img.set_visible(false);
    }
    if let Some(ref img) = *fallback_image.borrow() {
        img.set_visible(true);
    }
}

/// Sets the fallback icon on the `gtk4::Image`.
/// Uses the per-theme `preview_icon` if non-empty, otherwise falls back to `global_fallback_icon`.
/// Resolves the Nerd Font name to a GTK icon name and loads it via `Image::from_icon_name`.
fn set_fallback_icon(fallback_image: &Rc<RefCell<Option<Image>>>, preview_icon: &str, global_fallback_icon: &str) {
    let icon_name = if preview_icon.is_empty() { global_fallback_icon } else { preview_icon };
    if let Some(ref img) = *fallback_image.borrow() {
        if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
            img.set_icon_name(Some(&gtk_icon_name));
        }
    }
}

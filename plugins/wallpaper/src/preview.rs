use gtk4::Picture;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use tracing::debug;

/// Loads a preview image from the given path into the `gtk4::Picture`.
/// Falls back to showing the fallback icon text in the label if the image cannot be loaded.
pub fn update_preview(picture: &Rc<RefCell<Option<Picture>>>, label: &Rc<RefCell<Option<gtk4::Label>>>, preview_path: &str, fallback_icon: &str) {
    if preview_path.is_empty() {
        set_fallback_icon(label, fallback_icon);
        return;
    }

    let path = Path::new(preview_path);
    if !path.exists() {
        debug!("wallpaper widget: preview image not found: {}", preview_path);
        set_fallback_icon(label, fallback_icon);
        return;
    }

    if let Some(ref pic) = *picture.borrow() {
        pic.set_filename(Some(path));
    }
}

/// Sets the fallback icon text on the label.
fn set_fallback_icon(label: &Rc<RefCell<Option<gtk4::Label>>>, fallback_icon: &str) {
    if let Some(ref lbl) = *label.borrow() {
        lbl.set_text(fallback_icon);
    }
}

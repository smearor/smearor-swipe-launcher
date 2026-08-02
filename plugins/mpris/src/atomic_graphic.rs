use crate::atomic::MprisAtomicWidget;
use smearor_render_utils::draw_image_cover;
use smearor_render_utils::resolve_file_uri;
use smearor_swipe_launcher_plugin_api::AtomicGraphicRenderer;
use tracing::debug;

impl AtomicGraphicRenderer for MprisAtomicWidget {
    fn render_graphic(&self, pixels: &mut [u8], width: u32, height: u32) -> bool {
        let status = self.latest_status.borrow();
        let Some(status) = status.as_ref() else {
            return false;
        };

        if !status.has_player {
            return false;
        }

        let Some(metadata) = status.metadata.as_ref() else {
            return false;
        };

        let Some(art_url) = metadata.art_url.as_ref() else {
            return false;
        };

        let art_path = resolve_file_uri(art_url.as_str());
        debug!("MprisAtomicWidget: loading album art from '{}'", art_path);

        if draw_image_cover(pixels, width, height, &art_path) {
            debug!("MprisAtomicWidget: album art rendered {}x{}", width, height);
            true
        } else {
            debug!("MprisAtomicWidget: failed to load album art from '{}'", art_path);
            false
        }
    }
}

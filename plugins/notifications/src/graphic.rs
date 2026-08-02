use crate::labels::NotificationLabel;
use crate::widget::NotificationView;
use crate::widget::NotificationWidget;
use smearor_notifications_model::NotificationInfo;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use tracing::trace;

impl GraphicRenderer for NotificationWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("NotificationWidget: render_graphic {}x{}", width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        fill_background(&mut pixels, width, height, background_color(false));

        let text_col = text_color(false);
        let override_data = self.personalization.borrow().clone();
        let view = *self.current_view.borrow();

        let status = self.last_status.borrow();
        let (unread_count, do_not_disturb, notifications) = match &*status {
            Some(s) => (s.unread_count, s.do_not_disturb, &s.notifications),
            None => (0, false, &stabby::vec::Vec::<NotificationInfo>::new()),
        };

        let icon_char = if do_not_disturb { '\u{f1e6}' } else { '\u{f0f3}' };

        let icon_size = (width.min(height) as f32 * 0.5).min(40.0);
        draw_nerd_font_codepoint(&mut pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, text_col);

        match view {
            NotificationView::Compact => {
                let count_text = format!("{}", unread_count);
                draw_text_centered(
                    &mut pixels,
                    width,
                    height,
                    &count_text,
                    height as f32 * 0.72,
                    (height as f32 * 0.22).min(16.0).max(10.0),
                    text_col,
                );
            }
            NotificationView::Expanded => {
                let label = NotificationLabel::Notifications.localized_label(override_data.effective_locale());
                draw_text_centered(&mut pixels, width, height, label, height as f32 * 0.52, (height as f32 * 0.16).min(12.0).max(8.0), text_col);
                let count_text = format!("{}", unread_count);
                draw_text_centered(
                    &mut pixels,
                    width,
                    height,
                    &count_text,
                    height as f32 * 0.72,
                    (height as f32 * 0.22).min(16.0).max(10.0),
                    text_col,
                );
                if let Some(first) = notifications.first() {
                    let summary = first.summary.as_str();
                    draw_text_centered(&mut pixels, width, height, summary, height as f32 * 0.92, (height as f32 * 0.16).min(12.0).max(8.0), text_col);
                }
            }
        }

        FfiGraphic::from_pixels(width, height, pixels)
    }
}

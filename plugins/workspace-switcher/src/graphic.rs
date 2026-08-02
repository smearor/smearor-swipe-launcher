use crate::widget::WorkspaceSwitcherWidget;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_label_text;
use smearor_render_utils::draw_nerd_font_icon;
use smearor_render_utils::draw_progress_bar;
use smearor_render_utils::fill_background;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use tracing::debug;

impl GraphicRenderer for WorkspaceSwitcherWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        debug!("WorkspaceSwitcherWidget: render_graphic {}x{}", width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];

        let is_active = false;
        let bg = background_color(is_active);
        fill_background(&mut pixels, width, height, bg);

        let ws_list = self.workspaces.borrow();
        let icon_class = if ws_list.is_empty() {
            "nf-md-loading".to_string()
        } else {
            let idx = *self.current_view.borrow();
            let idx = idx.min(ws_list.len() - 1);
            let ws = &ws_list[idx];
            let key = ws.workspace_id.to_string();
            self.config.icon_map.get(&key).cloned().unwrap_or_else(|| self.config.default_icon.clone())
        };

        let icon_color = self.config.icon_config.icon_color().map(|c| c.to_rgba());
        draw_nerd_font_icon(&mut pixels, width, height, &icon_class, is_active, resolve_icon_codepoint, icon_color);

        if !self.config.icon_config.icon_only() {
            let ws_list = self.workspaces.borrow();
            if !ws_list.is_empty() {
                let idx = *self.current_view.borrow();
                let idx = idx.min(ws_list.len() - 1);
                let ws = &ws_list[idx];

                if self.config.show_label {
                    let label_text = ws.workspace_name.to_string();
                    if !label_text.is_empty() {
                        let main_color = self.config.text_colors.main_text_color().map(|c| c.to_rgba());
                        draw_label_text(&mut pixels, width, height, &label_text, is_active, main_color);
                    }
                }

                if ws_list.len() > 1 {
                    let fraction = idx as f32 / (ws_list.len() - 1) as f32;
                    let bar_color = smearor_render_utils::COLOR_TEXT;
                    draw_progress_bar(&mut pixels, width, height, fraction, bar_color);
                }
            } else {
                let main_color = self.config.text_colors.main_text_color().map(|c| c.to_rgba());
                draw_label_text(&mut pixels, width, height, "...", is_active, main_color);
            }
        }

        FfiGraphic::from_pixels(width, height, pixels)
    }
}

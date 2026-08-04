use crate::labels::PowerLabel;
use crate::widget::PowerWidget;
use crate::widget::WidgetView;
use smearor_power_model::PowerAction;
use smearor_power_model::power_action_icon;
use smearor_power_model::power_action_icon_unicode;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_icon_grid;
use smearor_render_utils::draw_nerd_font_codepoint;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use tracing::trace;

impl GraphicRenderer for PowerWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("PowerWidget: render_graphic {}x{}", width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let view = *self.widget_view.borrow();
        let override_data = self.personalization.borrow().clone();
        let locale = override_data.locale;

        match view {
            WidgetView::Compact => {
                render_compact(&mut pixels, width, height, self, &override_data);
            }
            WidgetView::Confirm => {
                render_confirm(&mut pixels, width, height, self, &locale);
            }
        }

        FfiGraphic::from_pixels(width, height, pixels)
    }
}

fn render_compact(pixels: &mut [u8], width: u32, height: u32, widget: &PowerWidget, override_data: &crate::personalization::PersonalizationOverride) {
    let bg = background_color(false);
    fill_background(pixels, width, height, bg);

    let text_col = widget
        .config
        .text_colors
        .main_text_color()
        .map(|c| c.to_rgba())
        .unwrap_or_else(|| text_color(false));
    let info_text_col = widget.config.text_colors.info_text_color().map(|c| c.to_rgba()).unwrap_or(text_col);
    let icon_col = widget.config.icon_config.icon_color().map(|c| c.to_rgba()).unwrap_or(text_col);

    let actions = widget.enabled_actions.borrow();
    let view_idx = *widget.current_view.borrow();
    let action = actions.get(view_idx).cloned().unwrap_or(PowerAction::Shutdown);

    let icon_char = resolve_icon_codepoint(power_action_icon(&action)).unwrap_or(power_action_icon_unicode(&action).chars().next().unwrap_or('\u{f0425}'));

    let icon_size = widget.config.icon_config.icon_size() as f32;
    draw_nerd_font_codepoint(pixels, width, height, icon_char, width as f32 / 2.0, height as f32 * 0.35, icon_size, icon_col);

    let label = PowerLabel::from_action(&action, override_data.locale);
    if !widget.config.icon_config.icon_only() {
        draw_text_centered(pixels, width, height, &label, height as f32 * 0.72, (height as f32 * 0.22).min(16.0).max(10.0), text_col);
    }

    let status = widget.last_status.borrow();
    if let Some(ref status) = *status {
        let info_text = if status.countdown_active {
            let label = PowerLabel::countdown_label(status.countdown_action, override_data.locale);
            PowerLabel::format_with_seconds(&label, status.countdown_remaining_seconds)
        } else {
            status.scheduled_info_text(override_data.effective_time_format())
        };
        if !info_text.is_empty() {
            draw_text_centered(
                pixels,
                width,
                height,
                &info_text,
                height as f32 * 0.92,
                (height as f32 * 0.16).min(12.0).max(8.0),
                info_text_col,
            );
        }
    }
}

fn render_confirm(pixels: &mut [u8], width: u32, height: u32, widget: &PowerWidget, locale: &smearor_swipe_launcher_plugin_api::Locale) {
    let bg = background_color(false);
    fill_background(pixels, width, height, bg);

    let confirm_actions = widget.confirm_actions();
    let icon_names: Vec<String> = confirm_actions.iter().map(|a| power_action_icon(a).to_string()).collect();
    let icon_refs: Vec<&str> = icon_names.iter().map(|s| s.as_str()).collect();

    draw_icon_grid(pixels, width, height, &icon_refs, 2, false, resolve_icon_codepoint);

    let text_col = text_color(false);
    let labels: Vec<String> = confirm_actions.iter().map(|a| PowerLabel::from_action(a, *locale)).collect();
    let rows = ((confirm_actions.len() + 1) / 2).max(1);
    let cell_h = height / rows as u32;
    for (i, label) in labels.iter().enumerate() {
        let col = i as u32 % 2;
        let row = i as u32 / 2;
        let _center_x = col * (width / 2) + (width / 2) / 2;
        let baseline_y = (row * cell_h) as f32 + cell_h as f32 * 0.85;
        draw_text_centered(pixels, width, height, label, baseline_y, (cell_h as f32 * 0.25).min(12.0).max(8.0), text_col);
    }
}

impl PowerLabel {
    /// Maps a `PowerAction` to its corresponding `PowerLabel` and localizes it.
    pub fn from_action(action: &PowerAction, locale: smearor_swipe_launcher_plugin_api::Locale) -> String {
        match action {
            PowerAction::Shutdown => PowerLabel::Shutdown.localized_label(locale),
            PowerAction::Reboot => PowerLabel::Reboot.localized_label(locale),
            PowerAction::Suspend => PowerLabel::Suspend.localized_label(locale),
            PowerAction::Hibernate => PowerLabel::Hibernate.localized_label(locale),
            PowerAction::Lock => PowerLabel::Lock.localized_label(locale),
            PowerAction::Logout => PowerLabel::Logout.localized_label(locale),
            PowerAction::RebootToFirmware => PowerLabel::Firmware.localized_label(locale),
            PowerAction::Cancel => PowerLabel::Cancel.localized_label(locale),
        }
    }
}

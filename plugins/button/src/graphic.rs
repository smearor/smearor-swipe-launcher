use crate::widget::ButtonWidget;
use smearor_render_utils::background_color;
use smearor_render_utils::draw_label_text;
use smearor_render_utils::draw_nerd_font_icon;
use smearor_render_utils::draw_text_centered;
use smearor_render_utils::fill_background;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_render_utils::text_color;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::GraphicRenderer;
use tracing::trace;

impl GraphicRenderer for ButtonWidget {
    fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
        trace!("ButtonWidget: render_graphic {}x{}", width, height);

        let mut pixels = vec![0u8; (width * height * 4) as usize];

        let state = self.internal_state.borrow();
        let is_active = state
            .as_ref()
            .map(|s| is_state_truthy_graphic(s, &self.config.state_icon))
            .unwrap_or(self.config.active);

        let bg = background_color(is_active);
        fill_background(&mut pixels, width, height, bg);

        let icon_name = self.resolve_current_icon();
        if let Some(icon) = &icon_name {
            let icon_color = self.config.icon_config.icon_color().map(|c| c.to_rgba());
            draw_nerd_font_icon(&mut pixels, width, height, icon, is_active, resolve_icon_codepoint, icon_color);
        }

        if !self.config.icon_config.icon_only() || icon_name.is_none() {
            let label_text = self.resolve_current_label();
            if !label_text.is_empty() {
                let main_color = self.config.text_colors.main_text_color().map(|c| c.to_rgba());
                draw_label_text(&mut pixels, width, height, &label_text, is_active, main_color);
            }
        }

        if !self.config.icon_config.icon_only() || icon_name.is_none() {
            if !self.config.info_text.is_empty() {
                let info_color = self
                    .config
                    .text_colors
                    .info_text_color()
                    .map(|c| c.to_rgba())
                    .unwrap_or_else(|| text_color(is_active));
                let font_size = (height as f32 * 0.16).min(12.0).max(8.0);
                draw_text_centered(&mut pixels, width, height, &self.config.info_text, height as f32 * 0.92, font_size, info_color);
            }
        }

        FfiGraphic::from_pixels(width, height, pixels)
    }
}

impl ButtonWidget {
    fn resolve_current_icon(&self) -> Option<String> {
        let state = self.internal_state.borrow();
        if let (Some(icon_expr), Some(state_value)) = (&self.config.state_icon, state.as_ref()) {
            return Some(resolve_state_expression_graphic(icon_expr, state_value));
        }
        self.config.icon.clone()
    }

    fn resolve_current_label(&self) -> String {
        let state = self.internal_state.borrow();
        if let Some(label_expr) = &self.config.state_label {
            if let Some(state_value) = state.as_ref() {
                return resolve_state_format_graphic(label_expr, state_value);
            }
        }
        if let Some(fallback) = &self.config.label_fallback {
            return fallback.clone();
        }
        self.config.main_text.clone()
    }
}

fn is_state_truthy_graphic(state: &serde_json::Value, state_icon: &Option<String>) -> bool {
    if let Some(icon_expr) = state_icon {
        if icon_expr.starts_with('{') {
            let inner = &icon_expr[1..icon_expr.len().saturating_sub(1)];
            if let Some((condition, _)) = inner.split_once('?') {
                return is_truthy_graphic(&state[condition]);
            }
        }
    }
    is_truthy_graphic(state)
}

fn is_truthy_graphic(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        serde_json::Value::String(s) => !s.is_empty() && s != "false" && s != "0",
        serde_json::Value::Null => false,
        _ => true,
    }
}

fn resolve_state_expression_graphic(expr: &str, state: &serde_json::Value) -> String {
    if !expr.starts_with('{') {
        return expr.to_string();
    }
    let inner = &expr[1..expr.len().saturating_sub(1)];
    if let Some((condition, values)) = inner.split_once('?') {
        let (true_val, false_val) = values.split_once(':').unwrap_or((values, ""));
        let field_value = &state[condition];
        if is_truthy_graphic(field_value) {
            true_val.to_string()
        } else {
            false_val.to_string()
        }
    } else {
        state[inner].as_str().unwrap_or(expr).to_string()
    }
}

fn resolve_state_format_graphic(format: &str, state: &serde_json::Value) -> String {
    let mut result = format.to_string();
    if let Some(object) = state.as_object() {
        for (key, value) in object {
            let replacement = match value {
                serde_json::Value::Number(number) => {
                    if let Some(integer) = number.as_i64() {
                        format!("{}", integer)
                    } else if let Some(float) = number.as_f64() {
                        format!("{}", float)
                    } else {
                        String::new()
                    }
                }
                serde_json::Value::String(string) => string.clone(),
                _ => value.to_string(),
            };
            result = result.replace(&format!("{{{}}}", key), &replacement);
        }
    }
    result
}

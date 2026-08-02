use crate::labels::WallpaperLabel;
use crate::widget::WallpaperWidget;
use crate::widget::WidgetView;
use smearor_render_utils::html::html_expanded_close;
use smearor_render_utils::html::html_expanded_open;
use smearor_swipe_launcher_plugin_api::WebRenderer;
use smearor_wallpaper_model::WallpaperThemeInfo;

impl WebRenderer for WallpaperWidget {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> String {
        let view = *self.widget_view.borrow();
        let override_data = self.personalization.borrow().clone();
        let locale = override_data.locale;

        match view {
            WidgetView::Compact => render_html_compact(self, instance_id, plugin_id, &locale),
            WidgetView::Grid => render_html_grid(self, instance_id, plugin_id, &locale),
        }
    }
}

fn render_html_compact(widget: &WallpaperWidget, instance_id: &str, plugin_id: &str, locale: &smearor_swipe_launcher_plugin_api::Locale) -> String {
    let status = widget.latest_status.borrow();
    let theme_info: Option<WallpaperThemeInfo> = status.as_ref().and_then(|s| s.themes.get(s.selected_theme_index).cloned());

    let (theme_name, preview_path) = match &theme_info {
        Some(theme) => (theme.name.to_string(), theme.preview_image_path.to_string()),
        None => (WallpaperLabel::NoTheme.localized_label(*locale), String::new()),
    };

    let mut html = html_expanded_open(plugin_id, "wallpaper");
    html.pop();
    html.push_str(&format!(
        r#" data-action-source="true" data-instance-id="{}" data-click-action="click" data-longpress-action="longpress" data-swipe-actions="true">"#,
        instance_id
    ));

    if !preview_path.is_empty() {
        html.push_str(&format!(
            r#"<div class="smearor-wallpaper-thumbnail" style="background-image: url('{}')"></div>"#,
            preview_path
        ));
    } else {
        let color_style = if let Some(color) = widget.config.icon_config.icon_color() {
            format!(
                r#" style="color: rgba({}, {}, {}, {});""#,
                (color.r * 255.0).round() as u8,
                (color.g * 255.0).round() as u8,
                (color.b * 255.0).round() as u8,
                color.a
            )
        } else {
            String::new()
        };
        html.push_str(&format!(
            r#"<div class="smearor-wallpaper-thumbnail smearor-wallpaper-thumbnail--fallback"><span class="nerd-icon nerd-nf-md-wallpaper"{}</span></div>"#,
            color_style
        ));
    }

    if !widget.config.icon_config.icon_only() {
        let main_color_style = if let Some(color) = widget.config.text_colors.main_text_color() {
            format!(
                r#" style="color: rgba({}, {}, {}, {}); opacity: 1;""#,
                (color.r * 255.0).round() as u8,
                (color.g * 255.0).round() as u8,
                (color.b * 255.0).round() as u8,
                color.a
            )
        } else {
            String::new()
        };
        html.push_str(&format!(r#"<div class="smearor-wallpaper-label"{}>{}</div>"#, main_color_style, theme_name));
    }

    html.push_str(html_expanded_close());
    html
}

fn render_html_grid(widget: &WallpaperWidget, _instance_id: &str, plugin_id: &str, _locale: &smearor_swipe_launcher_plugin_api::Locale) -> String {
    let status = widget.latest_status.borrow();
    let themes: Vec<WallpaperThemeInfo> = status.as_ref().map(|s| s.themes.iter().cloned().collect()).unwrap_or_default();
    let selected_index = status.as_ref().map(|s| s.selected_theme_index).unwrap_or(0);

    let mut html = html_expanded_open(plugin_id, "wallpaper");
    html.push_str(r#" data-action-source="true" data-view="grid">"#);
    html.push_str(r#"<div class="smearor-wallpaper-grid">"#);

    for (i, theme) in themes.iter().enumerate() {
        let is_selected = i == selected_index;
        let theme_name = theme.name.to_string();
        let preview_path = theme.preview_image_path.to_string();
        let selected_class = if is_selected { " smearor-wallpaper-grid-item--selected" } else { "" };

        let thumb_html = if !preview_path.is_empty() {
            format!(r#"<div class="smearor-wallpaper-grid-thumb" style="background-image: url('{}')"></div>"#, preview_path)
        } else {
            r#"<div class="smearor-wallpaper-grid-thumb smearor-wallpaper-grid-thumb--fallback"><span class="nerd-icon nerd-nf-md-wallpaper"></span></div>"#
                .to_string()
        };

        html.push_str(&format!(
            r#"<button class="smearor-wallpaper-grid-item{}" data-click-topic="service.wallpaper.command" data-click-payload='{{"action":"select_theme","theme_name":"{}"}}'>{}</button>"#,
            selected_class, theme_name, thumb_html
        ));
    }

    if themes.is_empty() {
        html.push_str(r#"<div class="smearor-wallpaper-grid-empty">No themes available</div>"#);
    }

    html.push_str("</div>");
    html.push_str(html_expanded_close());
    html
}

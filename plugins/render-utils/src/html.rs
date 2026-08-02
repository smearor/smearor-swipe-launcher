//! HTML helper functions for web rendering.
//!
//! These utilities generate HTML fragments for widgets that implement
//! `WebRenderer` for web instances.

/// Generate a compact button HTML fragment.
pub fn html_button(plugin_id: &str, icon_class: &str, label: &str, action: &str) -> String {
    format!(
        r#"<button class="smearor-widget smearor-button" data-plugin-id="{}" data-click-topic="tool.invoke" data-click-payload='{{"tool":"{}","action":"{}"}}'><span class="smearor-button-icon {}"></span><span class="smearor-button-label">{}</span></button>"#,
        plugin_id, plugin_id, action, icon_class, label
    )
}

/// Generate an expanded view container opening tag.
pub fn html_expanded_open(plugin_id: &str, widget_class: &str) -> String {
    format!(
        r#"<div class="smearor-widget smearor-{} smearor-widget--expanded" data-plugin-id="{}">"#,
        widget_class, plugin_id
    )
}

/// Generate a slider input HTML element.
pub fn html_slider(plugin_id: &str, value: u32, action: &str) -> String {
    format!(
        r#"<input type="range" min="0" max="100" value="{}" data-action="{}" data-plugin-id="{}" />"#,
        value, action, plugin_id
    )
}

/// Generate a list item HTML element.
pub fn html_list_item(icon_class: &str, text: &str, action: &str, plugin_id: &str) -> String {
    format!(
        r#"<div class="smearor-list-item" data-action="{}" data-plugin-id="{}"><span class="smearor-list-icon {}"></span><span class="smearor-list-text">{}</span></div>"#,
        action, plugin_id, icon_class, text
    )
}

/// Generate a closing `</div>` tag for expanded views.
pub fn html_expanded_close() -> &'static str {
    "</div>"
}

/// Sanitizes a string for use as a CSS class name.
///
/// Replaces every character outside `[a-zA-Z0-9_-]` with `-`, then collapses
/// multiple consecutive `-` into a single `-`.
///
/// # Examples
///
/// ```
/// use smearor_swipe_launcher_plugin_api::sanitize_css_class_name;
/// assert_eq!(sanitize_css_class_name("my.widget v2"), "my-widget-v2");
/// assert_eq!(sanitize_css_class_name("already_clean"), "already_clean");
/// assert_eq!(sanitize_css_class_name("a..b..c"), "a-b-c");
/// ```
pub fn sanitize_css_class_name(input: &str) -> String {
    let replaced: String = input
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '-' })
        .collect();

    let mut result = String::with_capacity(replaced.len());
    let mut prev_was_dash = false;
    for c in replaced.chars() {
        if c == '-' {
            if !prev_was_dash {
                result.push(c);
            }
            prev_was_dash = true;
        } else {
            result.push(c);
            prev_was_dash = false;
        }
    }
    result
}

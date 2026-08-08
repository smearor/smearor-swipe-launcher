/// Replaces `{key}` placeholders in the template with the corresponding values.
///
/// # Arguments
///
/// * `template` - The template string containing `{key}` placeholders.
/// * `replacements` - A slice of `(key, value)` pairs to substitute.
///
/// # Returns
///
/// The rendered string with all matching placeholders replaced.
pub fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (key, value) in replacements {
        result = result.replace(&format!("{{{key}}}"), value);
    }
    result
}

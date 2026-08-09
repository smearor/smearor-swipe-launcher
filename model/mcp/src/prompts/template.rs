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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_placeholder_replaced() {
        let result = render_template("Hello {name}", &[("name", "World")]);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn multiple_placeholders_replaced() {
        let result = render_template("{a} and {b}", &[("a", "1"), ("b", "2")]);
        assert_eq!(result, "1 and 2");
    }

    #[test]
    fn no_placeholders_unchanged() {
        let result = render_template("no placeholders here", &[]);
        assert_eq!(result, "no placeholders here");
    }

    #[test]
    fn missing_key_left_unchanged() {
        let result = render_template("Hello {name}", &[("other", "value")]);
        assert_eq!(result, "Hello {name}");
    }

    #[test]
    fn repeated_placeholders_all_replaced() {
        let result = render_template("{x} {x} {x}", &[("x", "yes")]);
        assert_eq!(result, "yes yes yes");
    }

    #[test]
    fn empty_value_replaces_placeholder() {
        let result = render_template("before{x}after", &[("x", "")]);
        assert_eq!(result, "beforeafter");
    }
}

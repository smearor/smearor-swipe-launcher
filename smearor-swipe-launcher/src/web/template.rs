use std::collections::HashMap;
use std::sync::Mutex;
use tracing::error;

/// Placeholder substitution marker in template files.
const PLACEHOLDER_PREFIX: &str = "{{";
const PLACEHOLDER_SUFFIX: &str = "}}";

/// The default HTML template used when no custom template is configured.
///
/// Compiled into the binary via `include_str!` from `resources/web/template-default.html`.
pub const DEFAULT_TEMPLATE: &str = include_str!("../../../resources/web/template-default.html");

/// Template engine for composing web instance pages.
///
/// Loads an HTML template file and replaces `{{placeholder}}` markers with
/// provided values. The host uses this to compose the full page from a
/// template and widget HTML fragments.
pub struct TemplateEngine {
    template_cache: Mutex<HashMap<String, String>>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self {
            template_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Load a template from a file path, caching it for subsequent calls.
    /// Falls back to the default template if the file cannot be read.
    pub fn load_template(&self, path: Option<&str>) -> String {
        let cache_key = path.unwrap_or("default").to_string();

        if let Ok(cache) = self.template_cache.lock() {
            if let Some(cached) = cache.get(&cache_key) {
                return cached.clone();
            }
        }

        let template_content = if let Some(path) = path {
            match std::fs::read_to_string(path) {
                Ok(content) => content,
                Err(e) => {
                    error!("Failed to load template from {}: {}, using default", path, e);
                    DEFAULT_TEMPLATE.to_string()
                }
            }
        } else {
            DEFAULT_TEMPLATE.to_string()
        };

        if let Ok(mut cache) = self.template_cache.lock() {
            cache.insert(cache_key, template_content.clone());
        }

        template_content
    }

    /// Render the template with the given placeholder values.
    pub fn render(&self, template: &str, placeholders: &HashMap<String, String>) -> String {
        let mut result = template.to_string();

        for (key, value) in placeholders {
            let marker = format!("{}{}{}", PLACEHOLDER_PREFIX, key, PLACEHOLDER_SUFFIX);
            result = result.replace(&marker, value);
        }

        result
    }

    /// Load and render in one step.
    pub fn load_and_render(&self, path: Option<&str>, placeholders: &HashMap<String, String>) -> String {
        let template = self.load_template(path);
        self.render(&template, placeholders)
    }

    /// Clear the template cache.
    #[allow(dead_code)]
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.template_cache.lock() {
            cache.clear();
        }
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

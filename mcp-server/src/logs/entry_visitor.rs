use tracing::field::Field;
use tracing::field::Visit;

/// Visitor that extracts the formatted message and structured fields from tracing event fields.
#[derive(Default)]
pub struct LogEntryVisitor {
    /// The formatted message extracted from the `message` field of a tracing event.
    pub message: String,
    /// Additional structured fields from the tracing event, formatted as `key=value` strings.
    pub fields: Vec<String>,
}

impl Visit for LogEntryVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.push(format!("{}={}", field.name(), value));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        } else {
            self.fields.push(format!("{}={:?}", field.name(), value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_empty() {
        let visitor = LogEntryVisitor::default();
        assert!(visitor.message.is_empty());
        assert!(visitor.fields.is_empty());
    }
}

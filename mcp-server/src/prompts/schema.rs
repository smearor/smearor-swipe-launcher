use std::collections::BTreeMap;

use serde::Serialize;

/// A single property in a prompt arguments JSON schema.
#[derive(Debug, Serialize)]
pub struct PromptArgumentProperty {
    /// The JSON schema type (e.g. "string", "number", "boolean").
    #[serde(rename = "type")]
    pub schema_type: &'static str,
    /// A human-readable description of the property.
    pub description: &'static str,
}

/// A JSON schema describing a prompt's arguments.
#[derive(Debug, Serialize)]
pub struct PromptArgumentsSchema {
    /// The JSON schema type, always "object" for prompt arguments.
    #[serde(rename = "type")]
    pub schema_type: &'static str,
    /// The named properties of the arguments object.
    pub properties: BTreeMap<&'static str, PromptArgumentProperty>,
    /// The list of required property names.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<&'static str>,
}

impl PromptArgumentsSchema {
    /// Create an empty arguments schema (no properties, no required fields).
    pub fn empty() -> Self {
        Self {
            schema_type: "object",
            properties: BTreeMap::new(),
            required: Vec::new(),
        }
    }

    /// Add a property to the schema.
    pub fn property(mut self, name: &'static str, schema_type: &'static str, description: &'static str) -> Self {
        self.properties.insert(name, PromptArgumentProperty { schema_type, description });
        self
    }

    /// Mark a property as required.
    pub fn required(mut self, name: &'static str) -> Self {
        self.required.push(name);
        self
    }

    /// Convert the schema to a `serde_json::Value`.
    pub fn to_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({"type": "object", "properties": {}}))
    }
}

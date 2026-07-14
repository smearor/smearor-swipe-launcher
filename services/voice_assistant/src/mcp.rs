use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::debug;

use crate::memory::FactCategory;
use crate::service::VoiceAssistantService;

impl VoiceAssistantService {
    /// Registers MCP resources and tools for the voice assistant service.
    pub fn register_mcp_capabilities(&self) {
        if !self.config.mcp_enabled {
            debug!("Voice Assistant Service: MCP tool registration disabled by config");
            return;
        }

        let broadcaster = self.get_broadcaster();

        let status_resource = RegisterResourceMessage::new(
            "voice_assistant://status",
            "Voice Assistant Status",
            "Current assistant state, transcript, and final answer.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(status_resource);

        let tool_catalog_resource = RegisterResourceMessage::new(
            "voice_assistant://tool_catalog",
            "Voice Assistant Tool Catalog",
            "List of all discovered tools in the catalog.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(tool_catalog_resource);

        let activate_tool = RegisterToolMessage::new(
            "voice_assistant_activate",
            "Starts audio capture and begins the voice pipeline.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(activate_tool);

        let deactivate_tool = RegisterToolMessage::new(
            "voice_assistant_deactivate",
            "Stops audio capture and cancels the pipeline.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(deactivate_tool);

        let submit_text_tool = RegisterToolMessage::new(
            "voice_assistant_submit_text",
            "Submits a text command directly (bypassing STT).",
            r#"{ "type": "object", "properties": { "text": { "type": "string", "description": "The text command to submit" } }, "required": ["text"] }"#,
        );
        broadcaster.broadcast_message_to_topic(submit_text_tool);

        let entities_resource = RegisterResourceMessage::new(
            "memory://entities",
            "Entity States",
            "Current states of all tracked entities (devices, apps, media).",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(entities_resource);

        let memory_query_tool = RegisterToolMessage::new(
            "memory_query",
            "Queries a specific entity by name or tool name from the entity state store.",
            r#"{ "type": "object", "properties": { "query": { "type": "string", "description": "Entity name or tool name to look up" } }, "required": ["query"] }"#,
        );
        broadcaster.broadcast_message_to_topic(memory_query_tool);

        let memory_store_tool = RegisterToolMessage::new(
            "memory_store",
            "Stores a fact in long-term semantic memory.",
            r#"{ "type": "object", "properties": { "key": { "type": "string", "description": "Short key for the fact" }, "value": { "type": "string", "description": "The fact content" }, "category": { "type": "string", "description": "Category: fact, preference, or habit" } }, "required": ["key", "value"] }"#,
        );
        broadcaster.broadcast_message_to_topic(memory_store_tool);

        let memory_store_batch_tool = RegisterToolMessage::new(
            "memory_store_batch",
            "Stores multiple facts in long-term semantic memory in a single batch call.",
            r#"{ "type": "object", "properties": { "facts": { "type": "array", "items": { "type": "object", "properties": { "key": { "type": "string", "description": "Short key for the fact" }, "value": { "type": "string", "description": "The fact content" }, "category": { "type": "string", "description": "Category: fact, preference, or habit" } }, "required": ["key", "value"] }, "description": "Array of facts to store" } }, "required": ["facts"] }"#,
        );
        broadcaster.broadcast_message_to_topic(memory_store_batch_tool);

        let memory_recall_tool = RegisterToolMessage::new(
            "memory_recall",
            "Recalls facts from long-term semantic memory by semantic similarity.",
            r#"{ "type": "object", "properties": { "query": { "type": "string", "description": "Natural language query to find related facts" }, "limit": { "type": "integer", "description": "Max number of facts to return (default: 3)" } }, "required": ["query"] }"#,
        );
        broadcaster.broadcast_message_to_topic(memory_recall_tool);

        let memory_list_tool = RegisterToolMessage::new(
            "memory_list",
            "Lists all stored fact keys in long-term memory.",
            r#"{ "type": "object", "properties": { "category": { "type": "string", "description": "Optional category filter: fact, preference, or habit" } }, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(memory_list_tool);

        let memory_forget_tool = RegisterToolMessage::new(
            "memory_forget",
            "Deletes a fact from long-term memory by key.",
            r#"{ "type": "object", "properties": { "key": { "type": "string", "description": "The key of the fact to delete" } }, "required": ["key"] }"#,
        );
        broadcaster.broadcast_message_to_topic(memory_forget_tool);
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("Voice Assistant Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();

        match tool_name.as_str() {
            "voice_assistant_activate" => {
                self.activate();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Voice assistant activated");
                broadcaster.broadcast_message_to_topic(response);
            }
            "voice_assistant_deactivate" => {
                self.deactivate();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Voice assistant deactivated");
                broadcaster.broadcast_message_to_topic(response);
            }
            "voice_assistant_submit_text" => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let text = args.get("text").and_then(|v| v.as_str());
                match text {
                    Some(text) => {
                        self.submit_text(text);
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "Text submitted");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: text");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            "memory_query" => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let store = self.entity_store.read().unwrap_or_else(|e| e.into_inner());
                let result = store
                    .iter()
                    .find(|(_, state)| state.name.to_lowercase().contains(&query.to_lowercase()) || state.tool.to_lowercase().contains(&query.to_lowercase()))
                    .map(|(_, state)| serde_json::to_string(state).unwrap_or_default());
                match result {
                    Some(json) => {
                        let response = InvokeToolResponse::success(&message.0.correlation_id, &json);
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Entity not found: {query}"));
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            "memory_store" => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
                let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");
                let category_str = args.get("category").and_then(|v| v.as_str()).unwrap_or("fact");
                let category = category_str.parse().unwrap_or(FactCategory::Fact);
                if key.is_empty() || value.is_empty() {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameters: key and value");
                    broadcaster.broadcast_message_to_topic(response);
                } else if let Ok(mut memory) = self.semantic_memory.write() {
                    match memory.store(key, value, category) {
                        Ok(id) => {
                            let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Fact stored with id: {id}"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                        Err(error) => {
                            let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Store failed: {error}"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                    }
                } else {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Memory lock poisoned");
                    broadcaster.broadcast_message_to_topic(response);
                }
            }
            "memory_recall" => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
                if let Ok(mut memory) = self.semantic_memory.write() {
                    match memory.recall(query, limit) {
                        Ok(facts) => {
                            let json = serde_json::to_string(
                                &facts
                                    .iter()
                                    .map(|f| serde_json::json!({"key": f.key, "value": f.value, "category": f.category.to_string()}))
                                    .collect::<Vec<_>>(),
                            )
                            .unwrap_or_default();
                            let response = InvokeToolResponse::success(&message.0.correlation_id, &json);
                            broadcaster.broadcast_message_to_topic(response);
                        }
                        Err(error) => {
                            let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Recall failed: {error}"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                    }
                } else {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Memory lock poisoned");
                    broadcaster.broadcast_message_to_topic(response);
                }
            }
            "memory_list" => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let category_str = args.get("category").and_then(|v| v.as_str());
                let category = category_str.and_then(|s| s.parse().ok());
                if let Ok(memory) = self.semantic_memory.read() {
                    match memory.list_keys(category.as_ref()) {
                        Ok(keys) => {
                            let json = serde_json::to_string(&keys).unwrap_or_default();
                            let response = InvokeToolResponse::success(&message.0.correlation_id, &json);
                            broadcaster.broadcast_message_to_topic(response);
                        }
                        Err(error) => {
                            let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("List failed: {error}"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                    }
                } else {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Memory lock poisoned");
                    broadcaster.broadcast_message_to_topic(response);
                }
            }
            "memory_forget" => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
                if key.is_empty() {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: key");
                    broadcaster.broadcast_message_to_topic(response);
                } else if let Ok(memory) = self.semantic_memory.read() {
                    match memory.forget(key) {
                        Ok(()) => {
                            let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Fact '{key}' forgotten"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                        Err(error) => {
                            let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Forget failed: {error}"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                    }
                } else {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Memory lock poisoned");
                    broadcaster.broadcast_message_to_topic(response);
                }
            }
            "memory_store_batch" => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let facts_json = match args.get("facts").and_then(|v| v.as_array()) {
                    Some(arr) if !arr.is_empty() => arr,
                    _ => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: facts (non-empty array)");
                        broadcaster.broadcast_message_to_topic(response);
                        return;
                    }
                };
                let facts: Vec<(String, String, FactCategory)> = facts_json
                    .iter()
                    .filter_map(|item| {
                        let key = item.get("key").and_then(|v| v.as_str())?;
                        let value = item.get("value").and_then(|v| v.as_str())?;
                        let category_str = item.get("category").and_then(|v| v.as_str()).unwrap_or("fact");
                        let category = category_str.parse().unwrap_or(FactCategory::Fact);
                        Some((key.to_string(), value.to_string(), category))
                    })
                    .collect();
                if facts.is_empty() {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "No valid facts in array");
                    broadcaster.broadcast_message_to_topic(response);
                } else if let Ok(mut memory) = self.semantic_memory.write() {
                    match memory.store_batch(&facts) {
                        Ok(ids) => {
                            let json = serde_json::to_string(&ids).unwrap_or_default();
                            let response = InvokeToolResponse::success(&message.0.correlation_id, &json);
                            broadcaster.broadcast_message_to_topic(response);
                        }
                        Err(error) => {
                            let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Batch store failed: {error}"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                    }
                } else {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Memory lock poisoned");
                    broadcaster.broadcast_message_to_topic(response);
                }
            }
            _ => {
                let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Unknown tool: {tool_name}"));
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, _sender_id: &str) {
        let uri = message.0.uri.to_string();
        debug!("Voice Assistant Service: InvokeResourceMessage uri={}", uri);
        let broadcaster = self.get_broadcaster();

        let response = match uri.as_str() {
            "voice_assistant://status" => {
                let state = self.state.read().map(|state| format!("{:?}", *state)).unwrap_or_else(|_| "Unknown".to_string());
                let json = serde_json::json!({
                    "state": state,
                    "transcript": self.current_transcript.read().map(|t| t.clone()).unwrap_or_default(),
                    "final_answer": self.current_answer.read().map(|a| a.clone()).unwrap_or_default(),
                });
                InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
            }
            "voice_assistant://tool_catalog" => {
                let catalog = self.tool_catalog.read().unwrap_or_else(|e| e.into_inner());
                let json = serde_json::json!({
                    "tools": catalog.iter().map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "description": t.description,
                            "input_schema": t.input_schema,
                        })
                    }).collect::<Vec<_>>(),
                });
                InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
            }
            "memory://entities" => {
                let store = self.entity_store.read().unwrap_or_else(|e| e.into_inner());
                let json = serde_json::json!({
                    "entities": store.values().collect::<Vec<_>>(),
                });
                InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
            }
            _ => InvokeResourceResponse::error(&message.0.correlation_id, &format!("Unknown resource: {uri}")),
        };
        broadcaster.broadcast_message_to_topic(response);
    }
}

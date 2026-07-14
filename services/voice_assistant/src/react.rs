use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::oneshot;
use tracing::debug;

use llama_cpp_4::model::LlamaChatMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_voice_assistant_model::LlmResponse;

use crate::memory::extract_entity_state;
use crate::service::VoiceAssistantService;

/// Errors that can occur during the voice assistant pipeline.
#[derive(Debug, thiserror::Error)]
pub enum AssistantError {
    /// LLM inference failed.
    #[error("LLM inference failed: {0}")]
    LlmInference(String),
    /// Tool invocation failed.
    #[error("Tool invocation failed: {0}")]
    ToolInvocation(String),
    /// Tool response timeout for correlation_id.
    #[error("Tool response timeout for correlation_id: {0}")]
    ToolTimeout(String),
    /// Max ReAct iterations reached without final answer.
    #[error("Max ReAct iterations reached without final answer")]
    MaxIterationsReached,
    /// LLM output could not be parsed.
    #[error("LLM output could not be parsed: {0}")]
    Parse(String),
}

/// Tracks pending tool invocations by correlation ID.
/// The `MessageHandler` implementation resolves the `oneshot::Sender`
/// when the matching `InvokeToolResponse` arrives.
pub type PendingInvocations = Arc<Mutex<HashMap<String, oneshot::Sender<String>>>>;

/// Parses the LLM output as either a tool call or a final answer.
pub fn parse_llm_response(output: &str) -> Result<LlmResponse, AssistantError> {
    let trimmed = output.trim();
    let json: serde_json::Value = serde_json::from_str(trimmed).map_err(|error| AssistantError::Parse(format!("Failed to parse JSON: {error}")))?;

    if let Some(tool) = json.get("tool").and_then(|v| v.as_str()) {
        let arguments = json.get("arguments").cloned().unwrap_or(serde_json::Value::Null);
        return Ok(LlmResponse::ToolCall {
            tool: tool.to_string(),
            arguments,
        });
    }

    if let Some(answer) = json.get("final_answer").and_then(|v| v.as_str()) {
        return Ok(LlmResponse::FinalAnswer { answer: answer.to_string() });
    }

    Err(AssistantError::Parse(format!("LLM output does not contain 'tool' or 'final_answer': {trimmed}")))
}

impl VoiceAssistantService {
    /// Executes the ReAct loop for a given user text input.
    ///
    /// Uses the persistent LLM worker (L0) for KV-cache reuse across commands.
    /// Separates persistent conversation history (L1) from transient context
    /// message and tool results to prevent stale context accumulation.
    pub async fn execute_react_loop(&self, user_text: &str) -> Result<String, AssistantError> {
        let _react_guard = crate::performance::TimingGuard::start(&self.performance_monitor, crate::performance::PerformanceMonitor::record_react_loop);
        let system_prompt = self.build_system_prompt();

        let worker = self
            .llm_worker
            .as_ref()
            .ok_or(AssistantError::LlmInference("LLM worker not initialized".to_string()))?;

        let max_tokens = worker.config().max_tokens;
        let max_iterations = self.config.max_react_iterations;

        // 0. Skip unconditional reset — the worker handles session management
        //    intelligently with rolling window trimming on context overflow.

        // 1. Load persistent conversation history (only real user/assistant messages).
        let mut conversation = self.conversation_history.read().map(|h| h.clone()).unwrap_or_default();

        // 2. Build transient context message (tools, entities, long-term facts).
        let context_message = self.build_context_message(user_text);

        // 3. Create a separate vector for the active LLM call.
        //    Tool results are appended ONLY to active_payload, never to conversation.
        let mut active_payload = Vec::with_capacity(conversation.len() + 2);
        active_payload.push(LlamaChatMessage::new("user".to_string(), context_message).map_err(|e| AssistantError::LlmInference(e.to_string()))?);
        active_payload.extend(conversation.iter().cloned());
        active_payload.push(LlamaChatMessage::new("user".to_string(), user_text.to_string()).map_err(|e| AssistantError::LlmInference(e.to_string()))?);

        // 4. Add the user message to persistent history.
        conversation.push(LlamaChatMessage::new("user".to_string(), user_text.to_string()).map_err(|e| AssistantError::LlmInference(e.to_string()))?);

        // 5. Run the ReAct loop via the worker.
        for iteration in 0..max_iterations {
            let llm_start = std::time::Instant::now();
            let (llm_output, trimmed_payload) = worker
                .generate(&system_prompt, active_payload.clone(), max_tokens)
                .await
                .map_err(|e| AssistantError::LlmInference(e.to_string()))?;
            self.performance_monitor.record_llm_inference(llm_start.elapsed());

            // Update active_payload if the worker trimmed it (rolling window).
            if trimmed_payload.len() != active_payload.len() {
                debug!("Voice Assistant: rolling window trimmed active_payload {} -> {}", active_payload.len(), trimmed_payload.len());
                active_payload = trimmed_payload;
            }

            match parse_llm_response(&llm_output) {
                Ok(LlmResponse::ToolCall { tool, arguments }) => {
                    // Add the LLM's tool call output to active_payload
                    // so the prompt stays consistent with the KV cache state.
                    active_payload.push(LlamaChatMessage::new("assistant".to_string(), llm_output).map_err(|e| AssistantError::LlmInference(e.to_string()))?);

                    // Invoke the tool asynchronously.
                    let tool_result = self.invoke_tool(&tool, &arguments).await?;

                    // Extract entity state from the tool call (L2).
                    self.update_entity_state(&tool, &arguments);

                    // Append tool result ONLY to active_payload (transient).
                    active_payload.push(
                        LlamaChatMessage::new(
                            "user".to_string(),
                            format!("Tool {tool} executed successfully. Result: {tool_result}\n\nThe tool call was successful. Now provide a final answer to the user. Respond with: {{\"final_answer\": \"<text>\"}}"),
                        )
                            .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                    );
                    debug!("Voice Assistant: ReAct iteration {iteration}: tool '{tool}' invoked, continuing");
                }
                Ok(LlmResponse::FinalAnswer { answer }) => {
                    // Append assistant message to persistent history.
                    conversation.push(LlamaChatMessage::new("assistant".to_string(), answer.clone()).map_err(|e| AssistantError::LlmInference(e.to_string()))?);

                    // Trim to last N messages.
                    let max_messages = self.config.max_history_messages;
                    if conversation.len() > max_messages {
                        let start = conversation.len() - max_messages;
                        conversation = conversation.split_off(start);
                    }

                    // Save trimmed history.
                    if let Ok(mut history) = self.conversation_history.write() {
                        *history = conversation;
                    }

                    self.performance_monitor.log_summary();
                    return Ok(answer);
                }
                Err(error) => {
                    debug!("Voice Assistant: ReAct parse error on iteration {iteration}: {error}");
                    if iteration + 1 < max_iterations {
                        active_payload
                            .push(LlamaChatMessage::new("assistant".to_string(), llm_output).map_err(|e| AssistantError::LlmInference(e.to_string()))?);
                        active_payload.push(
                            LlamaChatMessage::new(
                                "user".to_string(),
                                "Your previous response was not valid JSON. Please respond with ONLY a JSON object: either {\"tool\": \"<name>\", \"arguments\": {...}} or {\"final_answer\": \"<text>\"}.".to_string(),
                            )
                                .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                        );
                        continue;
                    }
                    return Err(error);
                }
            }
        }

        Err(AssistantError::MaxIterationsReached)
    }

    /// Updates the entity store and writes through to SQLite after a tool call.
    fn update_entity_state(&self, tool_name: &str, arguments: &serde_json::Value) {
        if let Some(state) = extract_entity_state(tool_name, arguments) {
            if let Ok(mut store) = self.entity_store.write() {
                store.insert(tool_name.to_string(), state.clone());
            }
            if let Ok(memory) = self.semantic_memory.read() {
                if let Err(error) = memory.write_entity_history(&state) {
                    debug!("Voice Assistant: entity history write failed: {error}");
                }
            }
        }
    }

    /// Invokes a tool via the MCP tool registry and waits for the response.
    ///
    /// Checks the tool result cache first. On cache miss, executes the tool
    /// via MCP and caches the result. Errors are classified into structured
    /// `ToolResult` with retryable flags.
    async fn invoke_tool(&self, tool_name: &str, arguments: &serde_json::Value) -> Result<String, AssistantError> {
        // Check tool result cache first.
        if let Some(cached) = self.tool_cache.get(tool_name, arguments) {
            self.performance_monitor.record_tool_cache_hit();
            if cached.success {
                debug!("Voice Assistant: tool cache hit for '{}' ({}ms)", tool_name, cached.execution_time_ms);
                return Ok(cached.result.unwrap_or_default());
            }
            // Cached failure — return the error if not retryable.
            if let Some(error) = cached.error {
                if !error.retryable {
                    debug!("Voice Assistant: tool cache hit (cached error) for '{}'", tool_name);
                    return Err(AssistantError::ToolInvocation(error.message));
                }
            }
        }
        self.performance_monitor.record_tool_cache_miss();

        let start_time = std::time::Instant::now();
        let correlation_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel::<String>();

        debug!("Voice Assistant: invoking tool '{}' with args: {} (correlation_id: {})", tool_name, arguments, correlation_id);

        {
            let mut pending = self
                .pending_invocations
                .lock()
                .map_err(|error| AssistantError::ToolInvocation(format!("Pending invocations lock poisoned: {error}")))?;
            pending.insert(correlation_id.clone(), tx);
        }

        let invoke_message = InvokeToolMessage::new(tool_name, &correlation_id, &arguments.to_string());

        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(invoke_message);

        let result = tokio::time::timeout(std::time::Duration::from_secs(10), rx)
            .await
            .map_err(|_error| {
                if let Ok(mut pending) = self.pending_invocations.lock() {
                    pending.remove(&correlation_id);
                }
                let assistant_error = AssistantError::ToolTimeout(correlation_id);
                let elapsed = start_time.elapsed();
                let tool_result = crate::tool_cache::error_to_tool_result(tool_name, &assistant_error, elapsed.as_millis() as u64);
                self.tool_cache.insert(tool_name, arguments, tool_result);
                assistant_error
            })?
            .map_err(|_error: _| {
                let assistant_error = AssistantError::ToolInvocation("Response channel closed".to_string());
                let elapsed = start_time.elapsed();
                let tool_result = crate::tool_cache::error_to_tool_result(tool_name, &assistant_error, elapsed.as_millis() as u64);
                self.tool_cache.insert(tool_name, arguments, tool_result);
                assistant_error
            })?;

        let elapsed = start_time.elapsed();
        let execution_time_ms = elapsed.as_millis() as u64;
        self.performance_monitor.record_tool_invocation(elapsed);

        // Cache the successful result.
        let tool_result = smearor_voice_assistant_model::ToolResult::success(tool_name, result.clone(), execution_time_ms);
        self.tool_cache.insert(tool_name, arguments, tool_result);

        Ok(result)
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolResponse>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolResponse>, _sender_id: &str) {
        let correlation_id = message.0.correlation_id.to_string();
        let result = message.0.result.to_string();

        if let Ok(mut pending) = self.pending_invocations.lock() {
            if let Some(sender) = pending.remove(&correlation_id) {
                let _ = sender.send(result);
            } else {
                debug!("Voice assistant: received tool response for unknown correlation_id: {}", correlation_id);
            }
        }
    }
}

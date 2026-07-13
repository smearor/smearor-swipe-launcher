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

use crate::service::VoiceAssistantService;

/// Errors that can occur during the voice assistant pipeline.
#[derive(Debug, thiserror::Error)]
pub enum AssistantError {
    /// Audio capture failed.
    #[error("Audio capture failed: {0}")]
    Audio(String),
    /// Speech-to-text failed.
    #[error("Speech-to-text failed: {0}")]
    Stt(String),
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
    pub async fn execute_react_loop(&self, user_text: &str) -> Result<String, AssistantError> {
        let system_prompt = self.build_system_prompt();
        let mut conversation =
            vec![LlamaChatMessage::new("user".to_string(), user_text.to_string()).map_err(|error| AssistantError::LlmInference(error.to_string()))?];

        let engine = self
            .llm_engine
            .as_ref()
            .ok_or(AssistantError::LlmInference("LLM engine not initialized".to_string()))?
            .clone();

        let max_iterations = self.config.max_react_iterations;
        let max_tokens = engine.config().max_tokens;
        let system_prompt_clone = system_prompt.clone();

        let (result, conversation) = tokio::task::spawn_blocking(move || -> Result<(String, Vec<LlamaChatMessage>), AssistantError> {
            let mut session = engine.create_session().map_err(|error| AssistantError::LlmInference(error.to_string()))?;
            let mut conversation = conversation;

            for iteration in 0..max_iterations {
                let llm_output = session
                    .generate(engine.model(), &system_prompt_clone, &conversation, max_tokens)
                    .map_err(|error| AssistantError::LlmInference(error.to_string()))?;

                match parse_llm_response(&llm_output) {
                    Ok(LlmResponse::ToolCall { tool, arguments }) => {
                        return Ok((format!("__TOOL_CALL__{}__{}", tool, arguments.to_string()), conversation));
                    }
                    Ok(LlmResponse::FinalAnswer { answer }) => {
                        return Ok((answer, conversation));
                    }
                    Err(error) => {
                        debug!("Voice Assistant: ReAct parse error on iteration {iteration}: {error}");
                        if iteration + 1 < max_iterations {
                            conversation.push(
                                LlamaChatMessage::new(
                                    "assistant".to_string(),
                                    llm_output,
                                )
                                    .map_err(|error| AssistantError::LlmInference(error.to_string()))?,
                            );
                            conversation.push(
                                LlamaChatMessage::new(
                                    "user".to_string(),
                                    "Your previous response was not valid JSON. Please respond with ONLY a JSON object: either {\"tool\": \"<name>\", \"arguments\": {...}} or {\"final_answer\": \"<text>\"}.".to_string(),
                                )
                                    .map_err(|error| AssistantError::LlmInference(error.to_string()))?,
                            );
                            continue;
                        }
                        return Err(error);
                    }
                }
            }

            Err(AssistantError::MaxIterationsReached)
        })
            .await
            .map_err(|join_error| AssistantError::LlmInference(format!("Blocking task failed: {join_error}")))??;

        // Check if the result is a tool call that needs async handling.
        if let Some(rest) = result.strip_prefix("__TOOL_CALL__") {
            let (tool_name, arguments_str) = rest
                .split_once("__")
                .ok_or_else(|| AssistantError::Parse("Invalid tool call format".to_string()))?;

            let arguments: serde_json::Value =
                serde_json::from_str(arguments_str).map_err(|error| AssistantError::Parse(format!("Invalid tool arguments: {error}")))?;

            // Invoke the tool via the MCP broker.
            let tool_result = self.invoke_tool(tool_name, &arguments).await?;

            // Re-enter the ReAct loop with the tool result appended.
            let mut conversation = conversation;
            conversation.push(
                LlamaChatMessage::new("user".to_string(), format!("Tool {tool_name} result: {tool_result}"))
                    .map_err(|error| AssistantError::LlmInference(error.to_string()))?,
            );

            // Recursive call for the next iteration.
            return self.execute_react_loop_with_conversation(&system_prompt, conversation).await;
        }

        Ok(result)
    }

    /// Executes the ReAct loop with an existing conversation history.
    async fn execute_react_loop_with_conversation(&self, system_prompt: &str, conversation: Vec<LlamaChatMessage>) -> Result<String, AssistantError> {
        let engine = self
            .llm_engine
            .as_ref()
            .ok_or(AssistantError::LlmInference("LLM engine not initialized".to_string()))?
            .clone();

        let max_iterations = self.config.max_react_iterations;
        let max_tokens = engine.config().max_tokens;
        let system_prompt_owned = system_prompt.to_string();

        let (result, conversation) = tokio::task::spawn_blocking(move || -> Result<(String, Vec<LlamaChatMessage>), AssistantError> {
            let mut session = engine.create_session().map_err(|error| AssistantError::LlmInference(error.to_string()))?;
            let mut conversation = conversation;

            for iteration in 0..max_iterations {
                let llm_output = session
                    .generate(engine.model(), &system_prompt_owned, &conversation, max_tokens)
                    .map_err(|error| AssistantError::LlmInference(error.to_string()))?;

                debug!("Voice Assistant: ReAct iteration {iteration} LLM output: {llm_output}");

                match parse_llm_response(&llm_output) {
                    Ok(LlmResponse::ToolCall { tool, arguments }) => {
                        return Ok((format!("__TOOL_CALL__{}__{}", tool, arguments.to_string()), conversation));
                    }
                    Ok(LlmResponse::FinalAnswer { answer }) => {
                        return Ok((answer, conversation));
                    }
                    Err(error) => {
                        debug!("Voice Assistant: ReAct parse error on iteration {iteration}: {error}");
                        if iteration + 1 < max_iterations {
                            conversation.push(
                                LlamaChatMessage::new(
                                    "assistant".to_string(),
                                    llm_output,
                                )
                                    .map_err(|error| AssistantError::LlmInference(error.to_string()))?,
                            );
                            conversation.push(
                                LlamaChatMessage::new(
                                    "user".to_string(),
                                    "Your previous response was not valid JSON. Please respond with ONLY a JSON object: either {\"tool\": \"<name>\", \"arguments\": {...}} or {\"final_answer\": \"<text>\"}.".to_string(),
                                )
                                    .map_err(|error| AssistantError::LlmInference(error.to_string()))?,
                            );
                            continue;
                        }
                        return Err(error);
                    }
                }
            }

            Err(AssistantError::MaxIterationsReached)
        })
            .await
            .map_err(|join_error| AssistantError::LlmInference(format!("Blocking task failed: {join_error}")))??;

        if let Some(rest) = result.strip_prefix("__TOOL_CALL__") {
            let (tool_name, arguments_str) = rest
                .split_once("__")
                .ok_or_else(|| AssistantError::Parse("Invalid tool call format".to_string()))?;

            let arguments: serde_json::Value =
                serde_json::from_str(arguments_str).map_err(|error| AssistantError::Parse(format!("Invalid tool arguments: {error}")))?;

            let tool_result = self.invoke_tool(tool_name, &arguments).await?;

            let mut new_conversation = conversation;
            new_conversation.push(
                LlamaChatMessage::new("user".to_string(), format!("Tool {tool_name} result: {tool_result}"))
                    .map_err(|error| AssistantError::LlmInference(error.to_string()))?,
            );

            return Box::pin(self.execute_react_loop_with_conversation(&system_prompt, new_conversation)).await;
        }

        Ok(result)
    }

    /// Invokes a tool via the MCP tool registry and waits for the response.
    ///
    /// This function registers a `oneshot::Sender` in the pending invocations
    /// tracker, broadcasts the `InvokeToolMessage`, and awaits the `Receiver`
    /// with a 10-second timeout.
    async fn invoke_tool(&self, tool_name: &str, arguments: &serde_json::Value) -> Result<String, AssistantError> {
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
            .map_err(|_| {
                if let Ok(mut pending) = self.pending_invocations.lock() {
                    pending.remove(&correlation_id);
                }
                AssistantError::ToolTimeout(correlation_id)
            })?
            .map_err(|_| AssistantError::ToolInvocation("Response channel closed".to_string()))?;

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

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::oneshot;
use tracing::debug;

use llama_cpp_4::model::LlamaChatMessage;
use llm_json::repair_json;
use regex::Regex;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_voice_assistant_model::LlmResponse;
use smearor_voice_assistant_model::NewInsight;
use smearor_voice_assistant_model::ToolResult;

use crate::llm::LlmWorker;
use crate::memory::FactCategory;
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

/// The outcome of a tool invocation, sent back through the pending invocations channel.
#[derive(Debug)]
pub struct ToolInvocationResult {
    /// The tool result as a JSON string. Empty on error.
    result: String,
    /// The error message. Empty when the invocation succeeded.
    error: String,
}

/// Tracks pending tool invocations by correlation ID.
/// The `MessageHandler` implementation resolves the `oneshot::Sender`
/// when the matching `InvokeToolResponse` arrives.
pub type PendingInvocations = Arc<Mutex<HashMap<String, oneshot::Sender<ToolInvocationResult>>>>;

/// The outcome of a resource read, sent back through the pending resource reads channel.
#[derive(Debug)]
pub struct ResourceInvocationResult {
    /// The resource contents as a JSON string. Empty on error.
    pub contents: String,
    /// The error message. Empty when the read succeeded.
    pub error: String,
}

/// Tracks pending resource reads by correlation ID.
/// The `MessageHandler` implementation resolves the `oneshot::Sender`
/// when the matching `InvokeResourceResponse` arrives.
pub type PendingResourceReads = Arc<Mutex<HashMap<String, oneshot::Sender<ResourceInvocationResult>>>>;

/// The outcome of a prompt invocation, sent back through the pending prompt invocations channel.
#[derive(Debug)]
pub struct PromptInvocationResult {
    /// The prompt response as a JSON string (serialized GetPromptResult). Empty on error.
    #[allow(dead_code)]
    pub result: String,
    /// The error message. Empty when the invocation succeeded.
    #[allow(dead_code)]
    pub error: String,
}

/// Tracks pending prompt invocations by correlation ID.
/// The `MessageHandler` implementation resolves the `oneshot::Sender`
/// when the matching `InvokePromptResponse` arrives.
pub type PendingPromptInvocations = Arc<Mutex<HashMap<String, oneshot::Sender<PromptInvocationResult>>>>;

/// Extracts all valid JSON objects from a string that may contain
/// surrounding text, control tokens (e.g. `<|im_start|>`), or multiple
/// JSON objects separated by garbage.
///
/// Escapes literal control characters (newlines, tabs, carriage returns) that
/// appear inside JSON string values. Small LLMs sometimes emit raw newlines
/// instead of `\n`, which makes `serde_json::from_str` reject otherwise valid
/// JSON. This function walks the string and escapes unescaped control chars
/// only inside string literals.
fn escape_newlines_in_json_strings(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_string = false;
    let mut escape = false;

    for ch in input.chars() {
        if escape {
            escape = false;
            result.push(ch);
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            result.push(ch);
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            result.push(ch);
            continue;
        }
        if in_string {
            match ch {
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                _ => result.push(ch),
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Uses brace counting to find balanced `{ ... }` blocks. Each candidate
/// is validated with `serde_json::from_str`. Non-JSON text between objects
/// (including ChatML control tokens) is silently skipped.
fn extract_all_json_objects(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    let bytes = text.as_bytes();
    let mut cursor = 0;

    while cursor < bytes.len() {
        // Find the next opening brace.
        let Some(rel_start) = text[cursor..].find('{') else {
            break;
        };
        let start = cursor + rel_start;

        // Walk forward, counting braces. Respect string literals to avoid
        // counting braces inside JSON string values.
        let mut depth: i32 = 0;
        let mut in_string = false;
        let mut escape = false;
        let mut end = None;

        for (i, &byte) in bytes[start..].iter().enumerate() {
            let pos = start + i;
            if escape {
                escape = false;
                continue;
            }
            if byte == b'\\' && in_string {
                escape = true;
                continue;
            }
            if byte == b'"' {
                in_string = !in_string;
                continue;
            }
            if in_string {
                continue;
            }
            if byte == b'{' {
                depth += 1;
            } else if byte == b'}' {
                depth -= 1;
                if depth == 0 {
                    end = Some(pos);
                    break;
                }
            }
        }

        let (candidate, next_cursor) = match end {
            Some(end_pos) => (&text[start..=end_pos], end_pos + 1),
            None => {
                // Unbalanced braces (depth > 0 at end of text): the LLM
                // produced truncated JSON with missing closing braces.
                // Pass the incomplete fragment to llm_json for repair.
                let fragment = &text[start..];
                if let Ok(repaired) = repair_json(fragment, &Default::default()) {
                    if serde_json::from_str::<serde_json::Value>(&repaired).is_ok() {
                        results.push(repaired);
                    }
                }
                break;
            }
        };

        if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
            results.push(candidate.to_string());
        } else {
            let escaped = escape_newlines_in_json_strings(candidate);
            if serde_json::from_str::<serde_json::Value>(&escaped).is_ok() {
                results.push(escaped);
            } else if let Ok(repaired) = repair_json(candidate, &Default::default()) {
                if serde_json::from_str::<serde_json::Value>(&repaired).is_ok() {
                    results.push(repaired);
                }
            }
        }

        cursor = next_cursor;
    }

    results
}

/// Extracts Chain-of-Thought (CoT) reasoning from `<|channel>thought\n...<channel|>`
/// blocks emitted by models like Gemma 4 12B.
///
/// Returns `(cot_text, remaining_output)` where `cot_text` is the reasoning
/// text (without the channel tokens) and `remaining_output` is the LLM output
/// with the CoT block removed.
fn extract_cot(output: &str) -> (Option<String>, String) {
    let start_token = "<|channel>thought";
    let end_token = "<channel|>";

    if let Some(start_pos) = output.find(start_token) {
        let after_start = &output[start_pos + start_token.len()..];
        if let Some(end_pos) = after_start.find(end_token) {
            let cot_text = after_start[..end_pos].trim().to_string();
            let before = &output[..start_pos];
            let after = &after_start[end_pos + end_token.len()..];
            let remaining = format!("{before}{after}").trim().to_string();
            return (Some(cot_text), remaining);
        } else {
            // Incomplete CoT block: <|channel>thought without closing <channel|>.
            // Strip the start token and any trailing text to avoid KV-cache
            // contamination with raw channel tokens.
            let before = &output[..start_pos];
            let remaining = before.trim().to_string();
            return (None, remaining);
        }
    }

    (None, output.to_string())
}

/// Sanitizes raw LLM output before JSON extraction.
///
/// Removes common chat-template control tokens and standalone role markers that
/// the model sometimes emits when no grammar enforces the output format.
fn sanitize_llm_output(output: &str) -> String {
    static MALFORMED_WITH_QUOTE_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    static MALFORMED_TOKEN_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

    let re_quote = MALFORMED_WITH_QUOTE_RE.get_or_init(|| Regex::new(r#"<\|im_(start|end|sep|turn)/\d+\\""#).unwrap());
    let re_token = MALFORMED_TOKEN_RE.get_or_init(|| Regex::new(r#"<\|im_(start|end|sep|turn)[^>\\}{]*>?"#).unwrap());

    let mut cleaned = output
        .replace("<|im_start|>", "")
        .replace("<|im_end|>", "")
        .replace("<|im_sep|>", "")
        .replace("<|im_turn|>", "")
        .replace("<|channel>thought", "")
        .replace("<channel|>", "")
        .replace("<|endoftext|>", "")
        .replace("<|startoftext|>", "")
        .replace("<end_of_turn>", "")
        .replace("<start_of_turn>", "")
        .replace("<|turn>", "")
        .replace("<turn|>", "")
        .replace("<|tool_call>", "")
        .replace("<tool_call|>", "")
        .replace("<|tool_response>", "")
        .replace("<tool_response|>", "")
        .replace("<|think>", "")
        .replace("<|think|>", "")
        .replace("## ", "");

    cleaned = re_quote.replace_all(&cleaned, "").to_string();
    cleaned = re_token.replace_all(&cleaned, "").to_string();

    cleaned = cleaned
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.eq_ignore_ascii_case("user")
                && !trimmed.eq_ignore_ascii_case("assistant")
                && !trimmed.eq_ignore_ascii_case("system")
                && !trimmed.eq_ignore_ascii_case("model")
        })
        .collect::<Vec<_>>()
        .join("\n");

    cleaned.trim().to_string()
}

/// Repairs common malformed JSON objects produced by small LLMs without grammar.
///
/// Example: `{"tool": "final_answer": "..."}` -> `{"final_answer": "..."}`
///
/// Also repairs unbalanced braces by inserting missing `}` before `]` or at
/// the end of the string. Small LLMs sometimes forget the closing `}` of the
/// last object in an array, producing e.g. `...}]} ` instead of `...}}]}`.
fn repair_malformed_json(json_str: &str) -> String {
    let trimmed = json_str.trim();
    if trimmed.starts_with("{\"tool\": \"final_answer\":") {
        return trimmed.replacen("{\"tool\": \"final_answer\":", "{\"final_answer\":", 1);
    }
    if trimmed.starts_with("{\"tool\": \"clarify\":") {
        return trimmed.replacen("{\"tool\": \"clarify\":", "{\"clarify\":", 1);
    }
    if trimmed.starts_with("{\"tool\": \"text_to_speech_answer\":") {
        return trimmed.replacen("{\"tool\": \"text_to_speech_answer\":", "{\"text_to_speech_answer\":", 1);
    }

    // Bracket-balance repair: count unmatched `{` (respecting string literals)
    // and insert missing `}` at the correct position.
    let mut in_string = false;
    let mut escape = false;
    let mut depth: i32 = 0;
    let mut last_close_brace_before_bracket: Option<usize> = None;
    let chars: Vec<char> = trimmed.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
        } else if ch == ']' && depth > 0 {
            // We hit a `]` while still having unclosed `{` — this is where
            // the missing `}` should be inserted (just before the `]`).
            last_close_brace_before_bracket = Some(i);
        }
    }

    if depth > 0 {
        let mut result = trimmed.to_string();
        if let Some(pos) = last_close_brace_before_bracket {
            // Insert `depth` closing braces before the `]` at `pos`.
            let byte_pos = trimmed.char_indices().nth(pos).map(|(b, _)| b);
            if let Some(bp) = byte_pos {
                let mut insert = String::new();
                for _ in 0..depth {
                    insert.push('}');
                }
                result.insert_str(bp, &insert);
                return result;
            }
        }
        // No `]` found with unclosed braces — append at end.
        for _ in 0..depth {
            result.push('}');
        }
        return result;
    }

    json_str.to_string()
}

/// Parses a single JSON string into an `LlmResponse`.
fn parse_single_json(json_str: &str) -> Result<LlmResponse, AssistantError> {
    let json: serde_json::Value = serde_json::from_str(json_str).map_err(|error| AssistantError::Parse(format!("Failed to parse JSON: {error}")))?;

    if let Some(tool) = json.get("tool").and_then(|v| v.as_str()) {
        let arguments = json.get("parameters").cloned().unwrap_or(serde_json::Value::Null);
        return Ok(LlmResponse::ToolCall {
            tool: tool.to_string(),
            arguments,
        });
    }

    if let Some(resource) = json.get("resource").and_then(|v| v.as_str()) {
        return Ok(LlmResponse::ResourceRead {
            resource: resource.to_string(),
        });
    }

    if let Some(answer) = json.get("final_answer").and_then(|v| v.as_str()) {
        let new_insights = json
            .get("new_insights")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let key = item.get("key").and_then(|v| v.as_str()).unwrap_or("");
                        let value = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
                        let category = item.get("category").and_then(|v| v.as_str()).unwrap_or("fact");
                        if key.is_empty() || value.is_empty() {
                            None
                        } else {
                            Some(NewInsight {
                                key: key.to_string(),
                                value: value.to_string(),
                                category: category.to_string(),
                            })
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        return Ok(LlmResponse::FinalAnswer {
            answer: answer.to_string(),
            new_insights,
        });
    }

    if let Some(question) = json.get("clarify").and_then(|v| v.get("question")).and_then(|v| v.as_str()) {
        return Ok(LlmResponse::Clarify {
            question: question.to_string(),
        });
    }

    if let Some(text) = json.get("text_to_speech_answer").and_then(|v| v.as_str()) {
        return Ok(LlmResponse::TextToSpeechAnswer { text: text.to_string() });
    }

    Err(AssistantError::Parse(format!(
        "LLM output does not contain 'tool', 'resource', 'final_answer', 'text_to_speech_answer', or 'clarify': {json_str}"
    )))
}

/// Parses the LLM output into a list of actions (tool calls, resource reads,
/// final answers, or clarifying questions).
///
/// The LLM may emit multiple JSON objects in a single response, separated by
/// control tokens or other text. Each valid JSON object is parsed independently.
/// If no valid JSON is found, returns a parse error.
pub fn parse_all_llm_responses(output: &str) -> Result<Vec<LlmResponse>, AssistantError> {
    let sanitized = sanitize_llm_output(output);

    // Fast path: if the entire sanitized output is a single valid JSON object,
    // parse it directly without going through extract_all_json_objects and
    // repair_json. Small LLMs sometimes produce valid JSON that repair_json
    // corrupts by inventing extra fields or array elements.
    let trimmed = sanitized.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(value) => {
                debug!("Voice Assistant: Fast path: serde_json parsed successfully, value type: {}", value);
                match parse_single_json(&value.to_string()) {
                    Ok(response) => {
                        debug!("Voice Assistant: Fast path: parse_single_json succeeded, returning directly");
                        return Ok(vec![response]);
                    }
                    Err(e) => {
                        debug!("Voice Assistant: Fast path: parse_single_json failed: {e}, falling through to extract_all_json_objects");
                    }
                }
            }
            Err(e) => {
                debug!("Voice Assistant: Fast path: serde_json::from_str failed: {e}");
                let repaired = repair_malformed_json(trimmed);
                if repaired != trimmed {
                    debug!("Voice Assistant: Fast path: retrying with repair_malformed_json");
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&repaired) {
                        if let Ok(response) = parse_single_json(&value.to_string()) {
                            debug!("Voice Assistant: Fast path: bracket-balance repair succeeded");
                            return Ok(vec![response]);
                        }
                    }
                }
                let escaped = escape_newlines_in_json_strings(trimmed);
                if escaped != trimmed {
                    debug!("Voice Assistant: Fast path: retrying with escape_newlines_in_json_strings");
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&escaped) {
                        if let Ok(response) = parse_single_json(&value.to_string()) {
                            debug!("Voice Assistant: Fast path: escaped retry succeeded");
                            return Ok(vec![response]);
                        }
                    }
                }
            }
        }
    } else {
        debug!(
            "Voice Assistant: Fast path: skipped — starts_with({})={}, ends_with({})={}",
            trimmed.chars().next().unwrap_or(' '),
            trimmed.starts_with('{'),
            trimmed.chars().last().unwrap_or(' '),
            trimmed.ends_with('}')
        );
    }

    let json_objects: Vec<String> = extract_all_json_objects(&sanitized)
        .into_iter()
        .map(|candidate| {
            let repaired = repair_malformed_json(&candidate);
            if serde_json::from_str::<serde_json::Value>(&repaired).is_ok() {
                repaired
            } else if let Ok(llm_repaired) = repair_json(&candidate, &Default::default()) {
                if serde_json::from_str::<serde_json::Value>(&llm_repaired).is_ok() {
                    llm_repaired
                } else {
                    candidate
                }
            } else {
                candidate
            }
        })
        .collect();
    if json_objects.is_empty() {
        return Err(AssistantError::Parse(format!("Failed to extract JSON from LLM output: {}", output.trim())));
    }
    let parsed: Vec<LlmResponse> = json_objects.iter().filter_map(|json_str| parse_single_json(json_str).ok()).collect();
    if parsed.is_empty() {
        return Err(AssistantError::Parse(format!("LLM output contained JSON but none matched known formats: {}", output.trim())));
    }
    Ok(parsed)
}

/// Serialize a planned LLM response back into the JSON format used in the
/// conversation so the model is reminded of the actions it originally
/// intended to take but could not be executed in the same turn.
fn format_llm_response_as_json(response: &LlmResponse) -> String {
    match response {
        LlmResponse::ToolCall { tool, arguments } => {
            format!("{{\"tool\": \"{tool}\", \"parameters\": {}}}", arguments)
        }
        LlmResponse::ResourceRead { resource } => {
            format!("{{\"resource\": \"{resource}\"}}")
        }
        LlmResponse::FinalAnswer { answer, new_insights } => {
            let escaped = answer.replace('\\', "\\\\").replace('"', "\\\"");
            if new_insights.is_empty() {
                format!("{{\"final_answer\": \"{escaped}\"}}")
            } else {
                let insights_json = serde_json::to_string(new_insights).unwrap_or_else(|_| "[]".to_string());
                format!("{{\"final_answer\": \"{escaped}\", \"new_insights\": {insights_json}}}")
            }
        }
        LlmResponse::Clarify { question } => {
            format!("{{\"clarify\": {{\"question\": \"{}\"}}}}", question.replace('\\', "\\\\").replace('"', "\\\""))
        }
        LlmResponse::TextToSpeechAnswer { text } => {
            format!("{{\"text_to_speech_answer\": \"{}\"}}", text.replace('\\', "\\\\").replace('"', "\\\""))
        }
    }
}

/// Build a reminder of the LLM's discarded plan for injection into the next
/// user message.
fn format_discarded_plan(responses: &[LlmResponse]) -> String {
    let json_items: Vec<String> = responses.iter().map(format_llm_response_as_json).collect();
    format!(
        "Note: Your previous response contained additional planned actions that were not executed because only one action is allowed per turn. Your original plan was:\n{}\n\nProceed with the next step from your plan if it still makes sense, but respond with exactly ONE JSON object.",
        json_items
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{}. {}", i + 1, item))
            .collect::<Vec<_>>()
            .join("\n")
    )
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

        // Reset last tool calls for this ReAct loop execution.
        if let Ok(mut calls) = self.last_tool_calls.write() {
            calls.clear();
        }

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

        // 5. Initialize training trace if training mode is active.
        let is_training = self.training_mode.lock().map(|m| *m).unwrap_or(false);
        if is_training {
            if let Ok(mut trace_guard) = self.active_trace.lock() {
                if let Some(trace) = trace_guard.as_mut() {
                    trace.user_text = user_text.to_string();
                    trace.start_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs_f64();
                    trace.steps.clear();
                    trace.success = None;
                    trace.end_time = None;
                }
            }
        }

        // 6. Run the ReAct loop via the worker.
        let mut last_tool_invoked: Option<(String, String)> = None;
        let mut last_resource_read: Option<String> = None;
        let mut rejected_final_answer = false;
        for iteration in 0..max_iterations {
            let llm_start = std::time::Instant::now();
            let (llm_output, trimmed_payload) = worker
                .generate(&system_prompt, active_payload.clone(), max_tokens, self.config.use_grammar)
                .await
                .map_err(|e| AssistantError::LlmInference(e.to_string()))?;
            self.performance_monitor.record_llm_inference(llm_start.elapsed());

            // Update active_payload if the worker trimmed it (rolling window).
            if trimmed_payload.len() != active_payload.len() {
                debug!("Voice Assistant: rolling window trimmed active_payload {} -> {}", active_payload.len(), trimmed_payload.len());
                active_payload = trimmed_payload;
            }

            // Extract Chain-of-Thought reasoning from <|channel>thought blocks
            // before parsing, so the CoT text is preserved in the training trace.
            let (cot_text, llm_output_without_cot) = extract_cot(&llm_output);
            if let Some(ref cot) = cot_text {
                debug!("Voice Assistant: ReAct iteration {iteration}: extracted CoT ({} chars)", cot.len());
            }
            let llm_output_for_parsing = if cot_text.is_some() { &llm_output_without_cot } else { &llm_output };

            let responses = match parse_all_llm_responses(llm_output_for_parsing) {
                Ok(responses) => responses,
                Err(error) => {
                    debug!("Voice Assistant: ReAct parse error on iteration {iteration}: {error}");
                    if is_training {
                        if let Ok(mut trace_guard) = self.active_trace.lock() {
                            if let Some(trace) = trace_guard.as_mut() {
                                trace.add_step(iteration as usize, &llm_output, cot_text.as_deref(), "parse_error", "", &error.to_string(), None);
                            }
                        }
                    }
                    if iteration + 1 < max_iterations {
                        active_payload.push(
                            LlamaChatMessage::new("assistant".to_string(), llm_output_without_cot.clone())
                                .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                        );
                        active_payload.push(
                            LlamaChatMessage::new(
                                "user".to_string(),
                                "Your previous response was not valid JSON. Please respond with ONLY a JSON object: either {\"tool\": \"<name>\", \"parameters\": {...}}, {\"resource\": \"<uri>\"}, {\"final_answer\": \"<text>\"}, or {\"clarify\": {{\"question\": \"...\"}}}.".to_string(),
                            )
                                .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                        );
                        continue;
                    }
                    // Last iteration: fall through to MaxIterationsReached
                    // instead of returning the raw parse error, so the service
                    // layer can handle it gracefully.
                    break;
                }
            };

            // Add the CoT-stripped LLM output as a single assistant message so
            // the prompt stays consistent with the KV cache state and no raw
            // channel tokens leak into the conversation history.
            active_payload
                .push(LlamaChatMessage::new("assistant".to_string(), llm_output_without_cot.clone()).map_err(|e| AssistantError::LlmInference(e.to_string()))?);

            // Execute only the first parsed action. Remaining planned actions
            // are preserved as a "discarded plan" and injected into the next
            // user message so the model remembers what it intended to do while
            // still observing the result of the first action.
            let discarded_plan = if responses.len() > 1 {
                debug!(
                    "Voice Assistant: ReAct iteration {iteration}: {} JSON objects parsed, executing only the first, preserving {} planned actions",
                    responses.len(),
                    responses.len() - 1
                );
                if is_training {
                    if let Ok(mut trace_guard) = self.active_trace.lock() {
                        if let Some(trace) = trace_guard.as_mut() {
                            let plan = format_discarded_plan(&responses[1..]);
                            trace.add_step(
                                iteration as usize,
                                &llm_output,
                                cot_text.as_deref(),
                                "extra_json_discarded",
                                &format!("{} additional JSON objects preserved as plan", responses.len() - 1),
                                &plan,
                                None,
                            );
                        }
                    }
                }
                Some(format_discarded_plan(&responses[1..]))
            } else {
                None
            };
            let mut short_circuit = None;
            let mut final_answer_text = None;
            match responses.into_iter().next() {
                None => continue,
                Some(response) => match response {
                    LlmResponse::ToolCall { tool, arguments } => {
                        // Prevent duplicate tool invocations (same tool AND same arguments).
                        let arguments_str = arguments.to_string();
                        if last_tool_invoked.as_ref() == Some(&(tool.clone(), arguments_str.clone())) {
                            debug!("Voice Assistant: ReAct iteration {iteration}: duplicate tool '{tool}' with identical arguments detected, skipping");
                            if is_training {
                                if let Ok(mut trace_guard) = self.active_trace.lock() {
                                    if let Some(trace) = trace_guard.as_mut() {
                                        trace.add_step(
                                            iteration as usize,
                                            &llm_output,
                                            cot_text.as_deref(),
                                            "duplicate_tool",
                                            &tool,
                                            "Tool already executed in previous step with same arguments",
                                            None,
                                        );
                                    }
                                }
                            }
                            active_payload.push(
                                LlamaChatMessage::new(
                                    "user".to_string(),
                                    format!("The tool '{tool}' was already executed in the previous step with the same arguments. Do not call it again with identical parameters. Either call a different tool, call the same tool with different arguments, ask a clarifying question with {{\"clarify\": {{\"question\": \"...\"}}}}, or provide a final answer with {{\"final_answer\": \"<text>\"}}."),
                                )
                                    .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                            );
                            continue;
                        }

                        // Invoke the tool asynchronously.
                        let tool_result = match self.invoke_tool(&tool, &arguments).await {
                            Ok(result) => result,
                            Err(error) => {
                                // Auto-fallback: if the tool was not found, check if the
                                // LLM hallucinated a tool name that matches a resource URI.
                                // E.g. `wallpaper_status` → resource `wallpaper://status`.
                                let error_str = error.to_string();
                                if error_str.contains("not found") || error_str.contains("not available") {
                                    if let Some(resource_uri) = self.find_resource_for_tool_name(&tool) {
                                        debug!(
                                            "Voice Assistant: ReAct iteration {iteration}: tool '{tool}' not found, \
                                             auto-falling back to resource '{resource_uri}'"
                                        );
                                        match self.invoke_resource(&resource_uri).await {
                                            Ok(resource_result) => {
                                                if is_training {
                                                    if let Ok(mut trace_guard) = self.active_trace.lock() {
                                                        if let Some(trace) = trace_guard.as_mut() {
                                                            trace.add_step(
                                                                iteration as usize,
                                                                &llm_output,
                                                                cot_text.as_deref(),
                                                                &format!("tool:{tool}→resource:{resource_uri}"),
                                                                &arguments.to_string(),
                                                                &resource_result,
                                                                None,
                                                            );
                                                        }
                                                    }
                                                }
                                                last_resource_read = Some(resource_uri.clone());
                                                const MAX_RESOURCE_RESULT_CHARS: usize = 2000;
                                                let display_result = if resource_result.len() > MAX_RESOURCE_RESULT_CHARS {
                                                    format!(
                                                        "{} ... [result truncated, {} chars total]",
                                                        &resource_result[..MAX_RESOURCE_RESULT_CHARS],
                                                        resource_result.len()
                                                    )
                                                } else {
                                                    resource_result
                                                };
                                                active_payload.push(
                                                    LlamaChatMessage::new(
                                                        "user".to_string(),
                                                        format!(
                                                            "Tool '{tool}' does not exist, but the resource '{resource_uri}' was read instead.\n\
                                                             Observation: {display_result}\n\n\
                                                             Note: for future requests of this type, use {{\"resource\": \"{resource_uri}\"}} directly.\n\
                                                             Now process the observation and respond with a final answer or next action."
                                                        ),
                                                    )
                                                    .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                                                );
                                                continue;
                                            }
                                            Err(resource_error) => {
                                                debug!(
                                                    "Voice Assistant: ReAct iteration {iteration}: \
                                                     resource fallback for '{resource_uri}' also failed: {resource_error}"
                                                );
                                            }
                                        }
                                    }
                                }

                                last_tool_invoked = Some((tool.clone(), arguments_str.clone()));
                                if is_training {
                                    if let Ok(mut trace_guard) = self.active_trace.lock() {
                                        if let Some(trace) = trace_guard.as_mut() {
                                            trace.add_step(
                                                iteration as usize,
                                                &llm_output,
                                                cot_text.as_deref(),
                                                &format!("tool:{tool}"),
                                                &arguments.to_string(),
                                                &format!("error: {error}"),
                                                None,
                                            );
                                        }
                                    }
                                }
                                let schema_hint = self.lookup_tool_schema(&tool);
                                active_payload.push(
                                    LlamaChatMessage::new(
                                        "user".to_string(),
                                        format!("Tool '{tool}' failed with error: {error}\n{schema_hint}\n\nThe tool call failed. Either try a different tool, read a resource with {{\"resource\": \"<uri>\"}}, ask a clarifying question with {{\"clarify\": {{\"question\": \"...\"}}}}, or inform the user with {{\"final_answer\": \"<text>\"}}."),
                                    )
                                        .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                                );
                                debug!("Voice Assistant: ReAct iteration {iteration}: tool '{tool}' failed: {error}");
                                continue;
                            }
                        };

                        // Record training step for successful tool call.
                        if is_training {
                            if let Ok(mut trace_guard) = self.active_trace.lock() {
                                if let Some(trace) = trace_guard.as_mut() {
                                    trace.add_step(
                                        iteration as usize,
                                        &llm_output,
                                        cot_text.as_deref(),
                                        &format!("tool:{tool}"),
                                        &arguments.to_string(),
                                        &tool_result,
                                        None,
                                    );
                                }
                            }
                        }

                        self.update_entity_state(&tool, &arguments);
                        last_tool_invoked = Some((tool.clone(), arguments_str.clone()));

                        if let Ok(mut calls) = self.last_tool_calls.write() {
                            calls.push(tool.clone());
                        }

                        const MAX_TOOL_RESULT_CHARS: usize = 2000;
                        // get_area_config returns structured JSON with exact button labels and
                        // actions. Summarizing it with a separate LLM call distorts the data,
                        // so the raw JSON is preserved even if it exceeds the default limit.
                        let display_result = if tool == "get_area_config" {
                            tool_result
                        } else if tool_result.len() > MAX_TOOL_RESULT_CHARS {
                            self.summarize_tool_result(&tool, &tool_result, user_text, worker, max_tokens)
                                .await
                                .unwrap_or_else(|error| {
                                    debug!("Voice Assistant: tool result summarization failed ({error}), falling back to truncation");
                                    format!(
                                        "{} ... [result truncated, {} chars total. If you need a specific entry, refine your query.]",
                                        &tool_result[..MAX_TOOL_RESULT_CHARS],
                                        tool_result.len()
                                    )
                                })
                        } else {
                            tool_result
                        };
                        let answer_hint = self.build_answer_hint(&tool, user_text);
                        let plan_note = discarded_plan.unwrap_or_default();
                        active_payload.push(
                            LlamaChatMessage::new(
                                "user".to_string(),
                                format!("Tool {tool} executed successfully. Result: {display_result}\n\n{plan_note}\n\nBased on the result, decide the next step: call another tool if needed (e.g. execute an app after searching), ask a clarifying question with {{\"clarify\": {{\"question\": \"...\"}}}}, or provide a final answer with {{\"final_answer\": \"<text>\"}}.{answer_hint}"),
                            )
                                .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                        );
                        debug!("Voice Assistant: ReAct iteration {iteration}: tool '{tool}' invoked, continuing");
                    }
                    LlmResponse::ResourceRead { resource } => {
                        if last_resource_read.as_deref() == Some(&resource) {
                            debug!("Voice Assistant: ReAct iteration {iteration}: duplicate resource '{resource}' detected, skipping");
                            if is_training {
                                if let Ok(mut trace_guard) = self.active_trace.lock() {
                                    if let Some(trace) = trace_guard.as_mut() {
                                        trace.add_step(
                                            iteration as usize,
                                            &llm_output,
                                            cot_text.as_deref(),
                                            "duplicate_resource",
                                            &resource,
                                            "Resource already read in previous step",
                                            None,
                                        );
                                    }
                                }
                            }
                            active_payload.push(
                                LlamaChatMessage::new(
                                    "user".to_string(),
                                    format!("The resource '{resource}' was already read in the previous step. Do not read it again. Try a different approach: call a tool with {{\"tool\": \"<name>\", \"arguments\": {{...}}}} to get the data you need, or reason about the information you already have and provide a final answer with {{\"final_answer\": \"<text>\"}}. Only if you cannot proceed, ask a clarifying question with {{\"clarify\": {{\"question\": \"...\"}}}}."),
                                )
                                    .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                            );
                            continue;
                        }

                        let resource_result = match self.invoke_resource(&resource).await {
                            Ok(result) => result,
                            Err(error) => {
                                last_resource_read = Some(resource.clone());
                                if is_training {
                                    if let Ok(mut trace_guard) = self.active_trace.lock() {
                                        if let Some(trace) = trace_guard.as_mut() {
                                            trace.add_step(
                                                iteration as usize,
                                                &llm_output,
                                                cot_text.as_deref(),
                                                &format!("resource:{resource}"),
                                                &resource,
                                                &format!("error: {error}"),
                                                None,
                                            );
                                        }
                                    }
                                }
                                let has_query_params = resource.contains('?') || resource.contains('&');
                                let tool_suggestions = if has_query_params {
                                    let matching_tools = self.find_tools_for_resource_scheme(&resource);
                                    if matching_tools.is_empty() {
                                        String::new()
                                    } else {
                                        format!(" Use a tool instead: {}.", matching_tools.join(", "))
                                    }
                                } else {
                                    String::new()
                                };
                                active_payload.push(
                                    LlamaChatMessage::new(
                                        "user".to_string(),
                                        format!("Resource '{resource}' failed with error: {error}\n\nThe resource URI does not exist. Resources do not accept query parameters (no '?' or '&').{tool_suggestions} Check the 'Available resources' list for valid URIs or the 'Available tools' list for actionable tools."),
                                    )
                                        .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                                );
                                debug!("Voice Assistant: ReAct iteration {iteration}: resource '{resource}' failed: {error}");
                                continue;
                            }
                        };

                        if is_training {
                            if let Ok(mut trace_guard) = self.active_trace.lock() {
                                if let Some(trace) = trace_guard.as_mut() {
                                    trace.add_step(
                                        iteration as usize,
                                        &llm_output,
                                        cot_text.as_deref(),
                                        &format!("resource:{resource}"),
                                        &resource,
                                        &resource_result,
                                        None,
                                    );
                                }
                            }
                        }

                        last_resource_read = Some(resource.clone());

                        const MAX_RESOURCE_RESULT_CHARS: usize = 2000;
                        let display_result = if resource_result.len() > MAX_RESOURCE_RESULT_CHARS {
                            format!(
                                "{} ... [result truncated, {} chars total]",
                                &resource_result[..MAX_RESOURCE_RESULT_CHARS],
                                resource_result.len()
                            )
                        } else {
                            resource_result
                        };
                        let plan_note = discarded_plan.unwrap_or_default();
                        active_payload.push(
                            LlamaChatMessage::new(
                                "user".to_string(),
                                format!("Resource {resource} read successfully. Result: {display_result}\n\n{plan_note}\n\nIf the result does not fully answer the user's question, try these steps in order:\n1. Check the 'Available tools' list for a tool that accepts parameters to get the specific data you need.\n2. Call that tool with {{\"tool\": \"<name>\", \"arguments\": {{...}}}}.\n3. If you already have enough information from the resource or a previous tool result, reason about it and provide a final answer with {{\"final_answer\": \"<text>\"}}.\n4. Only if you cannot proceed with any tool or reasoning, ask a clarifying question with {{\"clarify\": {{\"question\": \"...\"}}}}."),
                            )
                                .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                        );
                        debug!("Voice Assistant: ReAct iteration {iteration}: resource '{resource}' read, continuing");
                    }
                    LlmResponse::FinalAnswer { answer, new_insights } => {
                        if is_training {
                            if let Ok(mut trace_guard) = self.active_trace.lock() {
                                if let Some(trace) = trace_guard.as_mut() {
                                    let insights_summary = if new_insights.is_empty() {
                                        String::new()
                                    } else {
                                        format!(
                                            " | new_insights: [{}]",
                                            new_insights.iter().map(|i| format!("{}={}", i.key, i.value)).collect::<Vec<_>>().join(", ")
                                        )
                                    };
                                    trace.add_step(
                                        iteration as usize,
                                        &llm_output,
                                        cot_text.as_deref(),
                                        "final_answer",
                                        "",
                                        "",
                                        Some(&format!("{answer}{insights_summary}")),
                                    );
                                }
                            }
                        }
                        // Premature final_answer guard: if the user requested an
                        // action (start, open, launch, etc.) and no tool has been
                        // called yet, reject the final_answer once and redirect
                        // the LLM to use the appropriate tool.
                        if last_tool_invoked.is_none() && !rejected_final_answer && self.is_premature_action_final_answer(user_text) {
                            rejected_final_answer = true;
                            debug!(
                                "Voice Assistant: ReAct iteration {iteration}: \
                                 rejected premature final_answer for action request, \
                                 redirecting to tool execution"
                            );
                            if is_training {
                                if let Ok(mut trace_guard) = self.active_trace.lock() {
                                    if let Some(trace) = trace_guard.as_mut() {
                                        trace.add_step(
                                            iteration as usize,
                                            &llm_output,
                                            cot_text.as_deref(),
                                            "rejected_final_answer",
                                            "",
                                            "Premature final_answer rejected; redirecting to tool execution",
                                            None,
                                        );
                                    }
                                }
                            }
                            active_payload.push(
                                LlamaChatMessage::new(
                                    "user".to_string(),
                                    "[SYSTEM_ACTION]: CRITICAL: Your final_answer was rejected because the user requested an action (start, open, launch, etc.) but you have NOT called any tool yet. You MUST call the appropriate tool to perform the requested action. Do NOT return a final_answer without first executing the action via a tool call. Respond NOW with a tool call in the format {\"tool\": \"<name>\", \"parameters\": {...}}.".to_string(),
                                )
                                    .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                            );
                            continue;
                        }
                        // Automatically store any new insights in semantic memory.
                        if !new_insights.is_empty() {
                            if let Ok(mut memory) = self.semantic_memory.write() {
                                for insight in &new_insights {
                                    let category = insight.category.parse::<FactCategory>().unwrap_or(FactCategory::Fact);
                                    match memory.store(&insight.key, &insight.value, category) {
                                        Ok(id) => {
                                            debug!("Voice Assistant: Stored insight '{}' (id: {}): {}", insight.key, id, insight.value);
                                        }
                                        Err(error) => {
                                            debug!("Voice Assistant: Failed to store insight '{}': {}", insight.key, error);
                                        }
                                    }
                                }
                            }
                        }
                        debug!("Voice Assistant: ReAct iteration {iteration}: final_answer received, requesting TTS conversion");
                        final_answer_text = Some(answer.clone());
                        // Store only the final_answer JSON in conversation history.
                        // active_payload keeps the raw llm_output for KV cache
                        // consistency; conversation gets the cleaned version so
                        // future calls don't see a premature text_to_speech_answer.
                        let clean_output = format!("{{\"final_answer\": \"{}\"}}", answer.replace('\\', "\\\\").replace('"', "\\\""));
                        conversation
                            .push(LlamaChatMessage::new("assistant".to_string(), clean_output).map_err(|e| AssistantError::LlmInference(e.to_string()))?);
                        if self.config.tts.conversion_step {
                            active_payload.push(
                                LlamaChatMessage::new(
                                    "user".to_string(),
                                    format!("[SYSTEM_ACTION]: Convert your final_answer into a text_to_speech_answer. This text will be read aloud by a TTS engine — the user must understand it purely by listening.\n\
TTS rules:\n\
- Write ALL numbers as full words: \"zweiundzwanzig\" instead of \"22\", \"fünf Punkt eins\" instead of \"5.1\".\n\
- Replace ALL symbols with spoken words: \"Grad Celsius\" instead of \"°C\", \"Kilometer pro Stunde\" instead of \"km/h\", \"Prozent\" instead of \"%\".\n\
- Avoid abbreviations a TTS engine cannot pronounce naturally.\n\
- Do not use markdown, code blocks, or lists.\n\
- Focus only on what the user asked.\n\
- final_answer may contain digits and symbols. text_to_speech_answer MUST NOT contain any digits or symbols.\n\
Respond NOW with only {{\"text_to_speech_answer\": \"<spoken text>\"}}."),
                                )
                                    .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                            );
                        } else {
                            debug!("Voice Assistant: TTS conversion step disabled, returning final_answer directly");
                            short_circuit = Some(answer);
                        }
                    }
                    LlmResponse::TextToSpeechAnswer { text } => {
                        if is_training {
                            if let Ok(mut trace_guard) = self.active_trace.lock() {
                                if let Some(trace) = trace_guard.as_mut() {
                                    trace.add_step(iteration as usize, &llm_output, cot_text.as_deref(), "text_to_speech_answer", "", "", Some(&text));
                                    trace.success = Some(true);
                                }
                            }
                        }
                        debug!("Voice Assistant: ReAct iteration {iteration}: text_to_speech_answer received, completing");
                        short_circuit = Some(text);
                    }
                    LlmResponse::Clarify { question } => {
                        if is_training {
                            if let Ok(mut trace_guard) = self.active_trace.lock() {
                                if let Some(trace) = trace_guard.as_mut() {
                                    trace.add_step(iteration as usize, &llm_output, cot_text.as_deref(), "clarify", "", "", Some(&question));
                                }
                            }
                        }
                        debug!("Voice Assistant: ReAct iteration {iteration}: clarify received, requesting TTS conversion");
                        final_answer_text = Some(question.clone());
                        let clean_output = format!("{{\"clarify\": {{\"question\": \"{}\"}}}}", question.replace('\\', "\\\\").replace('"', "\\\""));
                        conversation
                            .push(LlamaChatMessage::new("assistant".to_string(), clean_output).map_err(|e| AssistantError::LlmInference(e.to_string()))?);
                        if self.config.tts.conversion_step {
                            active_payload.push(
                                LlamaChatMessage::new(
                                    "user".to_string(),
                                    format!("[SYSTEM_ACTION]: Convert your clarify question into a text_to_speech_answer. This text will be read aloud by a TTS engine — the user must understand it purely by listening.\n\
TTS rules:\n\
- Write ALL numbers as full words: \"zweiundzwanzig\" instead of \"22\", \"fünf Punkt eins\" instead of \"5.1\".\n\
- Replace ALL symbols with spoken words: \"Grad Celsius\" instead of \"°C\", \"Kilometer pro Stunde\" instead of \"km/h\", \"Prozent\" instead of \"%\".\n\
- Avoid abbreviations a TTS engine cannot pronounce naturally.\n\
- Do not use markdown, code blocks, or lists.\n\
- Focus only on what the user asked.\n\
Respond NOW with only {{\"text_to_speech_answer\": \"<spoken text>\"}}."),
                                )
                                    .map_err(|e| AssistantError::LlmInference(e.to_string()))?,
                            );
                        } else {
                            debug!("Voice Assistant: TTS conversion step disabled, returning clarify directly");
                            short_circuit = Some(question);
                        }
                    }
                },
            }

            // Handle short-circuit from FinalAnswer or Clarify.
            if let Some(result_text) = short_circuit {
                // Store the final_answer (not the TTS text) as previous_answer
                // in conversation history. This prevents the next turn from
                // seeing the spoken-form text and getting confused.
                let history_text = if let Some(ref fa) = final_answer_text {
                    format!("{{\"previous_answer\": \"{}\"}}", fa.replace('\\', "\\\\").replace('"', "\\\""))
                } else {
                    result_text.clone()
                };
                conversation.push(LlamaChatMessage::new("assistant".to_string(), history_text).map_err(|e| AssistantError::LlmInference(e.to_string()))?);

                let max_messages = self.config.max_history_messages;
                if conversation.len() > max_messages {
                    let start = conversation.len() - max_messages;
                    conversation = conversation.split_off(start);
                }

                if let Ok(mut history) = self.conversation_history.write() {
                    *history = conversation;
                }

                self.performance_monitor.log_summary();
                return Ok(result_text);
            }
        }

        // Record failed trace if training mode is active.
        if is_training {
            if let Ok(mut trace_guard) = self.active_trace.lock() {
                if let Some(trace) = trace_guard.as_mut() {
                    trace.success = Some(false);
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

    /// Looks up the input schema for a tool from the catalog and formats it
    /// as a hint string for the LLM when a tool call fails due to wrong parameters.
    fn lookup_tool_schema(&self, tool_name: &str) -> String {
        let catalog = self.tool_catalog.read().unwrap_or_else(|e| e.into_inner());
        let entry = catalog.iter().find(|t| t.name == tool_name);
        match entry {
            Some(entry) => {
                let schema = serde_json::from_str::<serde_json::Value>(&entry.input_schema).unwrap_or(serde_json::Value::Null);
                let params = schema
                    .get("properties")
                    .and_then(|p| p.as_object())
                    .map(|props| props.keys().map(|k| k.as_str()).collect::<Vec<_>>().join(", "))
                    .unwrap_or_default();
                let required = schema
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                    .unwrap_or_default();
                format!("Expected parameters for '{tool_name}': {params}\nRequired: {required}\nUse only these parameter names.")
            }
            None => String::new(),
        }
    }

    /// Builds a hint string that nudges the LLM toward providing a final answer
    /// when the tool result likely contains the answer to the user's question.
    /// This is a heuristic: tools whose names contain "get", "forecast", "status",
    /// "search", or "read" typically return data that answers the user directly.
    /// Execution tools (launch, exec, set, toggle, send) are excluded since they
    /// perform actions rather than return information.
    fn build_answer_hint(&self, tool_name: &str, user_text: &str) -> String {
        let is_info_tool = tool_name.contains("get_")
            || tool_name.contains("forecast")
            || tool_name.contains("status")
            || tool_name.contains("search_")
            || tool_name.contains("read_")
            || tool_name.contains("list_");
        let is_intermediate_tool = tool_name.contains("lookup_");
        let is_exec_tool = tool_name.contains("exec")
            || tool_name.contains("launch")
            || tool_name.contains("set_")
            || tool_name.contains("toggle")
            || tool_name.contains("send")
            || tool_name.contains("terminate")
            || tool_name.contains("delete")
            || tool_name.contains("create")
            || tool_name.contains("update");
        if is_info_tool && !is_exec_tool && !is_intermediate_tool {
            let user_lower = user_text.to_lowercase();
            let is_question = user_lower.contains("what")
                || user_lower.contains("wie")
                || user_lower.contains("welche")
                || user_lower.contains("welcher")
                || user_lower.contains("show")
                || user_lower.contains("zeige")
                || user_lower.contains("status")
                || user_lower.contains("current")
                || user_lower.contains("aktuell");
            if is_question {
                return "\n\n[SYSTEM_ACTION]: The tool result contains the answer. Summarize it in the user's language as a final_answer now.".to_string();
            }
        }
        if is_intermediate_tool {
            return "\n\n[SYSTEM_ACTION]: CRITICAL: This tool returned intermediate data, NOT the final answer. The task is INCOMPLETE. You MUST use this data in a subsequent tool call to retrieve the actual answer. Do NOT return a final_answer yet.".to_string();
        }
        String::new()
    }

    /// Detects whether a `final_answer` is premature because the user requested
    /// an action (start, open, launch, etc.) but no tool was called yet.
    /// Returns true when the user text contains action verbs AND the tool
    /// ranking includes exec tools with a score >= 0.4.
    fn is_premature_action_final_answer(&self, user_text: &str) -> bool {
        let user_lower = user_text.to_lowercase();
        let is_action_request = user_lower.contains("start")
            || user_lower.contains("öffn")
            || user_lower.contains("open")
            || user_lower.contains("launch")
            || user_lower.contains("beend")
            || user_lower.contains("schließen")
            || user_lower.contains("einschalten")
            || user_lower.contains("ausschalten")
            || user_lower.contains("schalte")
            || user_lower.contains("turn on")
            || user_lower.contains("turn off")
            || user_lower.contains("ausführen")
            || user_lower.contains("execute")
            || user_lower.contains("terminate")
            || user_lower.contains("install")
            || user_lower.contains("deinstall")
            || user_lower.contains("uninstall");
        if !is_action_request {
            return false;
        }
        let ranking = self.last_tool_ranking.read().map(|r| r.clone()).unwrap_or_default();
        ranking.iter().any(|(name, score)| {
            *score >= 0.4
                && (name.contains("exec")
                    || name.contains("launch")
                    || name.contains("start")
                    || name.contains("open")
                    || name.contains("terminate")
                    || name.contains("set_")
                    || name.contains("toggle")
                    || name.contains("send")
                    || name.contains("delete")
                    || name.contains("create")
                    || name.contains("update"))
        })
    }

    /// Finds tools whose names contain the scheme of a hallucinated resource URI.
    /// For example, "weather://current?lat=..." yields "weather", matching "weather_get_forecast".
    fn find_tools_for_resource_scheme(&self, resource_uri: &str) -> Vec<String> {
        let scheme = resource_uri.split("://").next().unwrap_or("");
        if scheme.is_empty() {
            return Vec::new();
        }
        let catalog = self.tool_catalog.read().unwrap_or_else(|e| e.into_inner());
        catalog
            .iter()
            .filter(|entry| entry.name.contains(scheme))
            .map(|entry| entry.name.clone())
            .collect()
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
        let (tx, rx) = oneshot::channel::<ToolInvocationResult>();

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

        // Check if the tool returned an error response.
        if !result.error.is_empty() {
            let tool_result = ToolResult::failure(tool_name, "TOOL_ERROR", result.error.clone(), true, execution_time_ms);
            self.tool_cache.insert(tool_name, arguments, tool_result);
            return Err(AssistantError::ToolInvocation(result.error));
        }

        // Cache the successful result.
        let tool_result = smearor_voice_assistant_model::ToolResult::success(tool_name, result.result.clone(), execution_time_ms);
        self.tool_cache.insert(tool_name, arguments, tool_result);

        Ok(result.result)
    }

    /// Summarizes a large tool result using a separate LLM call.
    ///
    /// This prevents context window overflow when tools return large payloads
    /// (e.g., full application lists). The summary preserves information
    /// relevant to the user's request while drastically reducing token count.
    async fn summarize_tool_result(
        &self,
        tool_name: &str,
        tool_result: &str,
        user_text: &str,
        worker: &Arc<LlmWorker>,
        max_tokens: usize,
    ) -> Result<String, AssistantError> {
        const MAX_SUMMARY_INPUT_CHARS: usize = 12000;
        const SUMMARY_MAX_TOKENS: usize = 512;

        let truncated_input = if tool_result.len() > MAX_SUMMARY_INPUT_CHARS {
            &tool_result[..MAX_SUMMARY_INPUT_CHARS]
        } else {
            tool_result
        };

        let summary_prompt = format!(
            "You are a tool result summarizer. The user asked: \"{user_text}\"\n\
             The tool \"{tool_name}\" returned the following result. Summarize it concisely, \
             keeping only information relevant to the user's request. If the result is a list \
             of items, include the most relevant entries with their key fields (e.g., name, path). \
             Do not add commentary — output only the summary.\n\n\
             Tool result:\n{truncated_input}"
        );

        let summary_messages = vec![LlamaChatMessage::new("user".to_string(), summary_prompt).map_err(|e| AssistantError::LlmInference(e.to_string()))?];

        let summary_system_prompt = "You are a concise summarizer. Output only the summary, nothing else.";
        let (summary_output, _) = worker
            .generate(summary_system_prompt, summary_messages, SUMMARY_MAX_TOKENS.min(max_tokens), false)
            .await
            .map_err(|e| AssistantError::LlmInference(e.to_string()))?;

        let summary = summary_output.trim().to_string();
        if summary.is_empty() {
            return Err(AssistantError::LlmInference("Tool result summary was empty".to_string()));
        }

        debug!(
            "Voice Assistant: summarized tool result for '{}' ({} chars -> {} chars)",
            tool_name,
            tool_result.len(),
            summary.len()
        );

        Ok(format!("{summary} [summarized from {total} chars]", total = tool_result.len()))
    }

    /// Reads an MCP resource by URI via the MCP resource protocol.
    /// Broadcasts an `InvokeResourceMessage` and waits for the matching
    /// `InvokeResourceResponse` on the pending resource reads channel.
    async fn invoke_resource(&self, uri: &str) -> Result<String, AssistantError> {
        // Validate the URI against the resource catalog before invoking.
        // This prevents the LLM from hallucinating URIs that don't exist.
        let known_uris: Vec<String> = self
            .resource_catalog
            .read()
            .map(|catalog| catalog.iter().map(|entry| entry.uri.clone()).collect())
            .unwrap_or_default();
        if !known_uris.contains(&uri.to_string()) {
            let available = known_uris.join(", ");
            debug!("Voice Assistant: rejected unknown resource URI '{}' — available: [{}]", uri, available);
            return Err(AssistantError::ToolInvocation(format!(
                "Unknown resource URI '{uri}'. Available resources: [{available}]. \
                 Use only URIs from the Available resources list in the context message."
            )));
        }

        let correlation_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel::<ResourceInvocationResult>();

        debug!("Voice Assistant: reading resource '{}' (correlation_id: {})", uri, correlation_id);

        {
            let mut pending = self
                .pending_resource_reads
                .lock()
                .map_err(|error| AssistantError::ToolInvocation(format!("Pending resource reads lock poisoned: {error}")))?;
            pending.insert(correlation_id.clone(), tx);
        }

        let resource_message = InvokeResourceMessage::new(uri, &correlation_id);

        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(resource_message);

        let result = tokio::time::timeout(std::time::Duration::from_secs(10), rx)
            .await
            .map_err(|_error| {
                if let Ok(mut pending) = self.pending_resource_reads.lock() {
                    pending.remove(&correlation_id);
                }
                AssistantError::ToolTimeout(correlation_id)
            })?
            .map_err(|_error: _| AssistantError::ToolInvocation("Resource response channel closed".to_string()))?;

        if !result.error.is_empty() {
            return Err(AssistantError::ToolInvocation(result.error));
        }

        Ok(result.contents)
    }

    /// Attempts to find a resource URI that matches a hallucinated tool name.
    ///
    /// When the LLM calls a non-existent tool like `wallpaper_status` instead of
    /// reading the resource `wallpaper://status`, this method searches the resource
    /// catalog for a match. Matching logic:
    /// - Exact name match (case-insensitive, underscores ↔ spaces)
    /// - URI suffix match (the part after `://`)
    /// - URI scheme + suffix (e.g. `wallpaper_status` → `wallpaper://status`)
    fn find_resource_for_tool_name(&self, tool_name: &str) -> Option<String> {
        let catalog = self.resource_catalog.read().ok()?;
        let normalized_tool = tool_name.to_lowercase().replace('_', " ");

        for entry in catalog.iter() {
            // Exact name match (case-insensitive, underscores ↔ spaces)
            let normalized_name = entry.name.to_lowercase().replace('_', " ");
            if normalized_name == normalized_tool {
                return Some(entry.uri.clone());
            }

            // URI suffix match: `wallpaper_status` matches `wallpaper://status`
            if let Some(suffix) = entry.uri.split("://").nth(1) {
                let normalized_suffix = suffix.to_lowercase().replace('_', " ");
                if normalized_suffix == normalized_tool {
                    return Some(entry.uri.clone());
                }

                // Also try scheme_suffix: `wallpaper_status` matches `wallpaper://status`
                let scheme = entry.uri.split("://").next().unwrap_or("");
                let combined = format!("{scheme}_{suffix}");
                let normalized_combined = combined.to_lowercase().replace('_', " ");
                if normalized_combined == normalized_tool {
                    return Some(entry.uri.clone());
                }
            }
        }
        None
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolResponse>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolResponse>, _sender_id: &str) {
        let correlation_id = message.0.correlation_id.to_string();
        let invocation_result = ToolInvocationResult {
            result: message.0.result.to_string(),
            error: message.0.error.to_string(),
        };

        if let Ok(mut pending) = self.pending_invocations.lock() {
            if let Some(sender) = pending.remove(&correlation_id) {
                let _ = sender.send(invocation_result);
            } else {
                debug!("Voice assistant: received tool response for unknown correlation_id: {}", correlation_id);
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceResponse>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceResponse>, _sender_id: &str) {
        let correlation_id = message.0.correlation_id.to_string();
        let resource_result = ResourceInvocationResult {
            contents: message.0.contents.to_string(),
            error: message.0.error.to_string(),
        };

        if let Ok(mut pending) = self.pending_resource_reads.lock() {
            if let Some(sender) = pending.remove(&correlation_id) {
                let _ = sender.send(resource_result);
            } else {
                debug!("Voice assistant: received resource response for unknown correlation_id: {}", correlation_id);
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokePromptResponse>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptResponse>, _sender_id: &str) {
        let correlation_id = message.0.correlation_id.to_string();
        let messages_json = if message.0.error.is_empty() {
            let messages: Vec<serde_json::Value> = message
                .0
                .messages
                .iter()
                .map(|m| serde_json::json!({"role": m.role.to_string(), "content": m.content.to_string()}))
                .collect();
            serde_json::to_string(&serde_json::json!({
                "description": "",
                "messages": messages
            }))
            .unwrap_or_else(|_| "{}".to_string())
        } else {
            String::new()
        };
        let prompt_result = PromptInvocationResult {
            result: messages_json,
            error: message.0.error.to_string(),
        };

        if let Ok(mut pending) = self.pending_prompt_invocations.lock() {
            if let Some(sender) = pending.remove(&correlation_id) {
                let _ = sender.send(prompt_result);
            } else {
                debug!("Voice assistant: received prompt response for unknown correlation_id: {}", correlation_id);
            }
        }
    }
}

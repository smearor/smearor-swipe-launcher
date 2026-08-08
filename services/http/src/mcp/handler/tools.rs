use crate::service::HttpService;
use crate::whitelist::is_url_allowed;
use smearor_http_model::HttpMcpTools;
use smearor_http_model::HttpRequestArgs;
use smearor_http_model::HttpRequestResponse;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use std::time::Duration;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for HttpService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("HTTP Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match HttpMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &message.0.correlation_id)));
                return;
            }
        };
        match tool {
            HttpMcpTools::HttpRequest => {
                let args: HttpRequestArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let method = args.method.as_deref().unwrap_or("GET").to_uppercase();
                let timeout_ms = args.timeout_ms.unwrap_or(self.config.default_timeout_ms);

                if !is_url_allowed(&args.url, &self.config.allowed_urls) {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("URL not whitelisted: {}", args.url));
                    broadcaster.broadcast_message_to_topic(response);
                    return;
                }

                let result =
                    execute_blocking_request(&args.url, &method, args.body.as_deref(), Duration::from_millis(timeout_ms), self.config.max_response_bytes);
                let response = match result {
                    Ok((status_code, response_body)) => {
                        let result = HttpRequestResponse {
                            status_code,
                            body: response_body,
                        };
                        let json = serde_json::to_string(&result).unwrap_or_default();
                        InvokeToolResponse::success(&message.0.correlation_id, &json)
                    }
                    Err(error) => InvokeToolResponse::error(&message.0.correlation_id, &format!("HTTP request failed: {error}")),
                };
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}

fn execute_blocking_request(url: &str, method: &str, body: Option<&str>, timeout: Duration, max_response_bytes: usize) -> Result<(u16, String), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let mut builder = match method {
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        _ => client.get(url),
    };

    if let Some(body) = body {
        builder = builder.body(body.to_string());
    }

    let response = builder.send().map_err(|e| format!("Request failed: {e}"))?;
    let status_code = response.status().as_u16();
    let bytes = response.bytes().map_err(|e| format!("Failed to read response body: {e}"))?;
    let truncated = bytes.iter().take(max_response_bytes).copied().collect::<Vec<u8>>();
    let body = String::from_utf8_lossy(&truncated).to_string();
    Ok((status_code, body))
}

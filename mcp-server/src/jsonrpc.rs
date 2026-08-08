//! Minimal JSON-RPC 2.0 types used by the MCP server transport layer.

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

/// A JSON-RPC request object.
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

/// A JSON-RPC response object.
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// A JSON-RPC error object.
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// JSON-RPC 2.0 parse error.
pub const JSONRPC_PARSE_ERROR: i32 = -32700;
/// JSON-RPC 2.0 invalid request.
#[allow(dead_code)]
pub const JSONRPC_INVALID_REQUEST: i32 = -32600;
/// JSON-RPC 2.0 method not found.
pub const JSONRPC_METHOD_NOT_FOUND: i32 = -32601;
/// JSON-RPC 2.0 invalid params.
pub const JSONRPC_INVALID_PARAMS: i32 = -32602;
/// JSON-RPC 2.0 internal error.
pub const JSONRPC_INTERNAL_ERROR: i32 = -32603;

impl JsonRpcResponse {
    /// Create a successful response.
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: Option<Value>, code: i32, message: String, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message, data }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).expect("valid request");
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, Some(serde_json::json!(1)));
        assert_eq!(request.method, "tools/list");
        assert!(request.params.is_some());
    }

    #[test]
    fn parse_notification_without_id() {
        let json = r#"{"jsonrpc":"2.0","method":"initialized"}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).expect("valid notification");
        assert!(request.id.is_none());
        assert_eq!(request.method, "initialized");
    }

    #[test]
    fn parse_mcp_initialized_notification() {
        let json = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).expect("valid notification");
        assert!(request.id.is_none());
        assert_eq!(request.method, "notifications/initialized");
    }

    #[test]
    fn success_response_serializes_result() {
        let response = JsonRpcResponse::success(Some(serde_json::json!(42)), serde_json::json!({"tools": []}));
        let json = serde_json::to_string(&response).expect("serializable");
        assert!(json.contains("\"result\""));
        assert!(json.contains("\"tools\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn error_response_serializes_error() {
        let response = JsonRpcResponse::error(Some(serde_json::json!(42)), JSONRPC_METHOD_NOT_FOUND, "Method not found".to_string(), None);
        let json = serde_json::to_string(&response).expect("serializable");
        assert!(json.contains("\"error\""));
        assert!(json.contains("Method not found"));
        assert!(!json.contains("\"result\""));
    }
}

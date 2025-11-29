use crate::gemini::{self, Options};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Input parameters for gemini tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GeminiArgs {
    /// Instruction for the task to send to gemini
    #[serde(rename = "PROMPT")]
    pub prompt: String,
    /// Run in sandbox mode. Defaults to `False`
    #[serde(default)]
    pub sandbox: bool,
    /// Resume the specified session of the gemini. Defaults to empty string, start a new session
    #[serde(rename = "SESSION_ID", default)]
    pub session_id: Option<String>,
    /// Return all messages (e.g. reasoning, tool calls, etc.) from the gemini session. Set to `False` by default, only the agent's final reply message is returned
    #[serde(default)]
    pub return_all_messages: bool,
    /// The model to use for the gemini session. This parameter is strictly prohibited unless explicitly specified by the user
    #[serde(default)]
    pub model: Option<String>,
}

/// Output from the gemini tool
#[derive(Debug, Serialize, schemars::JsonSchema)]
struct GeminiOutput {
    success: bool,
    #[serde(rename = "SESSION_ID")]
    session_id: String,
    agent_messages: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    all_messages: Option<Vec<HashMap<String, Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Clone)]
pub struct GeminiServer {
    tool_router: ToolRouter<GeminiServer>,
}

impl Default for GeminiServer {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl GeminiServer {
    /// Invokes the Gemini CLI to execute AI-driven tasks, returning structured JSON events and a session identifier for conversation continuity.
    ///
    /// **Return structure:**
    /// - `success`: boolean indicating execution status
    /// - `SESSION_ID`: unique identifier for resuming this conversation in future calls
    /// - `agent_messages`: concatenated assistant response text
    /// - `all_messages`: (optional) complete array of JSON events when `return_all_messages=True`
    /// - `error`: error description when `success=False`
    ///
    /// **Best practices:**
    /// - Always capture and reuse `SESSION_ID` for multi-turn interactions
    /// - Enable `sandbox` mode when file modifications should be isolated
    /// - Use `return_all_messages` only when detailed execution traces are necessary (increases payload size)
    /// - Only pass `model` when the user has explicitly requested a specific model
    #[tool(
        name = "gemini",
        description = "Invokes the Gemini CLI to execute AI-driven tasks, returning structured JSON events and a session identifier for conversation continuity."
    )]
    async fn gemini(
        &self,
        Parameters(args): Parameters<GeminiArgs>,
    ) -> Result<CallToolResult, McpError> {
        // Validate required parameters
        if args.prompt.is_empty() {
            return Err(McpError::invalid_params(
                "PROMPT is required and must be a non-empty string",
                None,
            ));
        }

        if let Some(ref model) = args.model {
            if model.is_empty() {
                return Err(McpError::invalid_params(
                    "Model overrides must be explicitly requested as a non-empty string",
                    None,
                ));
            }
        }

        // Convert empty string session_id to None
        let session_id = args.session_id.filter(|s| !s.is_empty());

        // Convert empty string model to None
        let model = args.model.filter(|m| !m.is_empty());

        // Create options for gemini client
        let opts = Options {
            prompt: args.prompt,
            sandbox: args.sandbox,
            session_id,
            return_all_messages: args.return_all_messages,
            model,
        };

        // Execute gemini
        let result = match gemini::run(opts).await {
            Ok(r) => r,
            Err(e) => {
                return Err(McpError::internal_error(
                    format!("Failed to execute gemini: {}", e),
                    None,
                ));
            }
        };

        // Prepare the response
        if result.success {
            let output = GeminiOutput {
                success: true,
                session_id: result.session_id,
                agent_messages: result.agent_messages,
                all_messages: if args.return_all_messages {
                    Some(result.all_messages)
                } else {
                    None
                },
                error: None,
            };

            let json_output = serde_json::to_string(&output).map_err(|e| {
                McpError::internal_error(format!("Failed to serialize output: {}", e), None)
            })?;

            Ok(CallToolResult::success(vec![Content::text(json_output)]))
        } else {
            let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
            Err(McpError::internal_error(error_msg, None))
        }
    }
}

#[tool_handler]
impl ServerHandler for GeminiServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides a gemini tool for AI-driven tasks. Use the gemini tool to execute tasks via the Gemini CLI.".to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_args_deserialization() {
        let json = r#"{
            "PROMPT": "test prompt",
            "sandbox": true,
            "SESSION_ID": "session-123",
            "return_all_messages": false,
            "model": "gemini-pro"
        }"#;

        let args: GeminiArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.prompt, "test prompt");
        assert!(args.sandbox);
        assert_eq!(args.session_id, Some("session-123".to_string()));
        assert!(!args.return_all_messages);
        assert_eq!(args.model, Some("gemini-pro".to_string()));
    }

    #[test]
    fn test_gemini_args_empty_session_id() {
        let json = r#"{
            "PROMPT": "test prompt",
            "SESSION_ID": ""
        }"#;

        let args: GeminiArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.session_id, Some("".to_string()));
    }

    #[test]
    fn test_gemini_output_serialization() {
        let output = GeminiOutput {
            success: true,
            session_id: "session-123".to_string(),
            agent_messages: "Hello, world!".to_string(),
            all_messages: None,
            error: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("SESSION_ID"));
        assert!(json.contains("agent_messages"));
        assert!(!json.contains("all_messages"));
    }
}

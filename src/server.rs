use crate::gemini::{self, Options};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde::Deserialize;

/// Input parameters for gemini tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GeminiArgs {
    /// Instruction for the task to send to gemini
    #[serde(rename = "PROMPT")]
    pub prompt: String,
    /// Resume the specified session of the gemini. If not provided or empty, starts a new session
    #[serde(rename = "SESSION_ID", default)]
    pub session_id: Option<String>,
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
    /// - `error`: error description when `success=False`
    ///
    /// **Best practices:**
    /// - Always capture and reuse `SESSION_ID` for multi-turn interactions
    #[tool(
        name = "gemini",
        description = "Invokes the Gemini CLI to execute AI-driven tasks, returning structured JSON events and a session identifier for conversation continuity."
    )]
    async fn gemini(
        &self,
        Parameters(args): Parameters<GeminiArgs>,
    ) -> Result<CallToolResult, McpError> {
        // Validate required parameters
        if args.prompt.trim().is_empty() {
            return Err(McpError::invalid_params(
                "PROMPT is required and must be a non-empty, non-whitespace string",
                None,
            ));
        }

        // Convert empty string session_id to None
        let session_id = args.session_id.filter(|s| !s.is_empty());

        // Create options for gemini client
        let opts = Options {
            prompt: args.prompt,
            session_id,
            additional_args: gemini::default_additional_args(),
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
            let mut response_text = format!(
                "success: true\nSESSION_ID: {}\nagent_messages: {}",
                result.session_id, result.agent_messages
            );

            Ok(CallToolResult::success(vec![Content::text(response_text)]))
        } else {
            let mut error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());

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
            "SESSION_ID": "session-123"
        }"#;

        let args: GeminiArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.prompt, "test prompt");
        assert_eq!(args.session_id, Some("session-123".to_string()));
    }

    #[test]
    fn test_gemini_args_empty_session_id_treated_as_none() {
        let json = r#"{
            "PROMPT": "test prompt",
            "SESSION_ID": ""
        }"#;

        let args: GeminiArgs = serde_json::from_str(json).unwrap();
        // Empty session_id is deserialized as Some(""), but will be filtered to None in the handler
        assert_eq!(args.session_id, Some("".to_string()));
    }
}

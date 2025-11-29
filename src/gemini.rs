use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

const PROMPT_DEPRECATION_WARNING: &str = "The --prompt (-p) flag has been deprecated";
const KEY_SESSION_ID: &str = "session_id";
const KEY_TYPE: &str = "type";
const KEY_ROLE: &str = "role";
const KEY_CONTENT: &str = "content";
const KEY_ERROR: &str = "error";
const KEY_MESSAGE: &str = "message";
const TYPE_MESSAGE: &str = "message";
const ROLE_ASSISTANT: &str = "assistant";

#[derive(Debug, Clone)]
pub struct Options {
    pub prompt: String,
    pub sandbox: bool,
    pub session_id: Option<String>,
    pub return_all_messages: bool,
    pub model: Option<String>,
}

#[derive(Debug)]
pub struct GeminiResult {
    pub success: bool,
    pub session_id: String,
    pub agent_messages: String,
    pub all_messages: Vec<HashMap<String, Value>>,
    pub error: Option<String>,
}

/// Escape prompt for Windows command line
fn windows_escape(prompt: &str) -> String {
    let mut result = prompt.replace('\\', "\\\\");
    result = result.replace('"', "\\\"");
    result = result.replace('\n', "\\n");
    result = result.replace('\r', "\\r");
    result = result.replace('\t', "\\t");
    result = result.replace('\x08', "\\b");
    result = result.replace('\x0c', "\\f");
    result = result.replace('\'', "\\'");
    result
}

/// Process a single JSON line from the gemini CLI output
fn process_json_line(
    line_data: &Value,
    result: &mut GeminiResult,
    return_all_messages: bool,
) {
    // Collect all messages if requested
    if return_all_messages {
        if let Ok(map) = serde_json::from_value::<HashMap<String, Value>>(line_data.clone()) {
            result.all_messages.push(map);
        }
    }

    // Extract session_id
    if let Some(session_id) = line_data.get(KEY_SESSION_ID).and_then(|v| v.as_str()) {
        if !session_id.is_empty() {
            result.session_id = session_id.to_string();
        }
    }

    // Extract agent messages
    let item_type = line_data.get(KEY_TYPE).and_then(|v| v.as_str()).unwrap_or("");
    let item_role = line_data.get(KEY_ROLE).and_then(|v| v.as_str()).unwrap_or("");

    if item_type == TYPE_MESSAGE && item_role == ROLE_ASSISTANT {
        if let Some(content) = line_data.get(KEY_CONTENT).and_then(|v| v.as_str()) {
            // Skip deprecation warning
            if content.contains(PROMPT_DEPRECATION_WARNING) {
                return;
            }
            result.agent_messages.push_str(content);
        }
    }

    // Check for errors
    if item_type.contains("fail") || item_type.contains("error") {
        result.success = false;
        if let Some(error_obj) = line_data.get(KEY_ERROR).and_then(|v| v.as_object()) {
            if let Some(msg) = error_obj.get(KEY_MESSAGE).and_then(|v| v.as_str()) {
                result.error = Some(format!("gemini error: {}", msg));
            }
        } else if let Some(msg) = line_data.get(KEY_MESSAGE).and_then(|v| v.as_str()) {
            result.error = Some(format!("gemini error: {}", msg));
        }
    }
}

/// Build the gemini command with the given options
fn build_command(opts: &Options) -> Command {
    let gemini_bin = std::env::var("GEMINI_BIN").unwrap_or_else(|_| "gemini".to_string());

    let mut cmd = Command::new(gemini_bin);
    cmd.arg("--prompt");

    // Escape prompt for Windows if needed
    let prompt = if cfg!(windows) {
        windows_escape(&opts.prompt)
    } else {
        opts.prompt.clone()
    };
    cmd.arg(&prompt);
    cmd.arg("-o");
    cmd.arg("stream-json");

    // Add optional flags
    if opts.sandbox {
        cmd.arg("--sandbox");
    }
    if let Some(ref model) = opts.model {
        cmd.args(["--model", model]);
    }
    if let Some(ref session_id) = opts.session_id {
        cmd.args(["--resume", session_id]);
    }

    // Configure process
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    cmd
}

/// Execute Gemini CLI with the given options and return the result
pub async fn run(opts: Options) -> Result<GeminiResult> {
    // Build and spawn the command
    let mut cmd = build_command(&opts);
    let mut child = cmd.spawn().context("Failed to spawn gemini command")?;

    // Read stdout
    let stdout = child.stdout.take().context("Failed to get stdout")?;
    let stderr = child.stderr.take().context("Failed to get stderr")?;

    let mut result = GeminiResult {
        success: true,
        session_id: String::new(),
        agent_messages: String::new(),
        all_messages: Vec::new(),
        error: None,
    };

    // Read stdout and stderr concurrently
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();
    let mut stderr_output = String::new();
    let mut parse_error_seen = false;
    let mut stdout_closed = false;
    let mut stderr_closed = false;
    while !stdout_closed || !stderr_closed {
        tokio::select! {
            line = stdout_reader.next_line(), if !stdout_closed => {
                let line = line.context("Failed to read from stdout")?;

                match line {
                    Some(line) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        // Parse JSON line
                        let line_data: Value = match serde_json::from_str(trimmed) {
                            Ok(data) => data,
                            Err(e) => {
                                if !parse_error_seen {
                                    record_parse_error(&mut result, &e, trimmed);
                                    parse_error_seen = true;
                                }
                                continue;
                            }
                        };

                        // Process the parsed JSON line
                        process_json_line(&line_data, &mut result, opts.return_all_messages);
                    }
                    None => stdout_closed = true,
                }
            }
            line = stderr_reader.next_line(), if !stderr_closed => {
                match line {
                    Ok(Some(line)) => {
                        if !stderr_output.is_empty() {
                            stderr_output.push('\n');
                        }
                        stderr_output.push_str(&line);
                    }
                    Ok(None) => stderr_closed = true,
                    Err(e) => {
                        eprintln!("Warning: Failed to read from stderr: {}", e);
                        stderr_closed = true;
                    }
                }
            }
        }
    }

    // Wait for process to finish
    let status = child
        .wait()
        .await
        .context("Failed to wait for gemini command")?;

    if !status.success() {
        result.success = false;
        let error_msg = if let Some(ref err) = result.error {
            err.clone()
        } else {
            format!("gemini command failed with exit code: {:?}", status.code())
        };

        if !stderr_output.is_empty() {
            result.error = Some(format!("{}\nStderr: {}", error_msg, stderr_output));
        } else {
            result.error = Some(error_msg);
        }
    }

    Ok(enforce_required_fields(result))
}

fn record_parse_error(result: &mut GeminiResult, error: &serde_json::Error, line: &str) {
    let parse_msg = format!("JSON parse error: {}. Line: {}", error, line);
    result.success = false;
    result.error = match result.error.take() {
        Some(existing) if !existing.is_empty() => Some(format!("{existing}\n{parse_msg}")),
        _ => Some(parse_msg),
    };
}

fn enforce_required_fields(mut result: GeminiResult) -> GeminiResult {
    let mut errors = Vec::new();

    if result.session_id.is_empty() {
        errors.push("Failed to get `SESSION_ID` from the gemini session.".to_string());
    }

    if result.agent_messages.is_empty() {
        errors.push("Failed to get `agent_messages` from the gemini session.\nYou can try to set `return_all_messages` to `True` to get the full information.".to_string());
    }

    if !errors.is_empty() {
        result.success = false;
        let new_error = errors.join("\n");
        let existing_error = result.error.take().filter(|s| !s.is_empty());
        result.error = match existing_error {
            Some(prev) => Some(format!("{}\n{}", prev, new_error)),
            None => Some(new_error),
        };
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_options_creation() {
        let opts = Options {
            prompt: "test prompt".to_string(),
            sandbox: false,
            session_id: None,
            return_all_messages: false,
            model: None,
        };

        assert_eq!(opts.prompt, "test prompt");
        assert!(!opts.sandbox);
    }

    #[test]
    fn test_options_with_session() {
        let opts = Options {
            prompt: "resume task".to_string(),
            sandbox: true,
            session_id: Some("test-session-123".to_string()),
            return_all_messages: true,
            model: Some("gemini-pro".to_string()),
        };

        assert_eq!(opts.session_id, Some("test-session-123".to_string()));
        assert_eq!(opts.model, Some("gemini-pro".to_string()));
        assert!(opts.return_all_messages);
        assert!(opts.sandbox);
    }

    #[test]
    fn test_windows_escape() {
        let prompt = "Hello\nWorld\t\"Test\"";
        let escaped = windows_escape(prompt);
        assert!(escaped.contains("\\n"));
        assert!(escaped.contains("\\t"));
        assert!(escaped.contains("\\\""));
    }

    #[test]
    fn test_record_parse_error_sets_failure_and_appends_message() {
        let mut result = GeminiResult {
            success: true,
            session_id: "session".to_string(),
            agent_messages: "ok".to_string(),
            all_messages: Vec::new(),
            error: Some("existing".to_string()),
        };

        let err = serde_json::from_str::<Value>("not-json").unwrap_err();
        record_parse_error(&mut result, &err, "not-json");

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("JSON parse error"));
        assert!(result.error.as_ref().unwrap().contains("existing"));
    }

    #[test]
    fn test_enforce_required_fields_requires_session_id() {
        let result = GeminiResult {
            success: true,
            session_id: String::new(),
            agent_messages: "msg".to_string(),
            all_messages: Vec::new(),
            error: None,
        };

        let updated = enforce_required_fields(result);

        assert!(!updated.success);
        assert!(updated
            .error
            .as_ref()
            .unwrap()
            .contains("Failed to get `SESSION_ID`"));
    }

    #[test]
    fn test_enforce_required_fields_requires_agent_messages() {
        let result = GeminiResult {
            success: true,
            session_id: "session".to_string(),
            agent_messages: String::new(),
            all_messages: Vec::new(),
            error: None,
        };

        let updated = enforce_required_fields(result);

        assert!(!updated.success);
        assert!(updated
            .error
            .as_ref()
            .unwrap()
            .contains("Failed to get `agent_messages`"));
    }

    #[test]
    fn test_build_command_basic() {
        let opts = Options {
            prompt: "test prompt".to_string(),
            sandbox: false,
            session_id: None,
            return_all_messages: false,
            model: None,
        };

        let cmd = build_command(&opts);
        let program = cmd.as_std().get_program();

        // Should use "gemini" as the binary name (or GEMINI_BIN env var)
        assert!(
            program == "gemini" || program.to_string_lossy().contains("gemini")
        );
    }

    #[test]
    fn test_build_command_with_all_options() {
        let opts = Options {
            prompt: "complex prompt".to_string(),
            sandbox: true,
            session_id: Some("session-123".to_string()),
            return_all_messages: true,
            model: Some("gemini-pro".to_string()),
        };

        let cmd = build_command(&opts);
        let program = cmd.as_std().get_program();

        // Should use "gemini" as the binary name
        assert!(
            program == "gemini" || program.to_string_lossy().contains("gemini")
        );
    }

    #[test]
    fn test_build_command_with_session_only() {
        let opts = Options {
            prompt: "resume".to_string(),
            sandbox: false,
            session_id: Some("abc-123".to_string()),
            return_all_messages: false,
            model: None,
        };

        let cmd = build_command(&opts);
        let program = cmd.as_std().get_program();

        assert!(
            program == "gemini" || program.to_string_lossy().contains("gemini")
        );
    }
}

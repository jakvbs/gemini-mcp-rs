use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

const PROMPT_DEPRECATION_WARNING: &str = "The --prompt (-p) flag has been deprecated";
const KEY_SESSION_ID: &str = "session_id";
const KEY_TYPE: &str = "type";
const KEY_ROLE: &str = "role";
const KEY_CONTENT: &str = "content";
const KEY_ERROR: &str = "error";
const KEY_MESSAGE: &str = "message";
const TYPE_MESSAGE: &str = "message";
const ROLE_ASSISTANT: &str = "assistant";
const DEFAULT_TIMEOUT_SECS: u64 = 600; // 10 minutes
const MAX_TIMEOUT_SECS: u64 = 3600; // 1 hour
const MAX_MESSAGES_LIMIT: usize = 10000; // Maximum number of messages to store
const MAX_NON_JSON_LINES: usize = 1000; // Maximum non-JSON lines to store
const MAX_STDERR_BYTES: usize = 100_000; // Maximum stderr output to capture (100KB)
const GEMINI_CONFIG_FILE: &str = "GEMINI.md"; // Configuration file name
const MAX_CONFIG_SIZE: usize = 100_000; // Maximum GEMINI.md file size (100KB)

#[derive(Debug, Clone, Deserialize)]
struct ServerConfig {
    #[serde(default)]
    additional_args: Vec<String>,
    timeout_secs: Option<u64>,
}

fn resolve_config_path() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("GEMINI_MCP_CONFIG_PATH") {
        let trimmed = env_path.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join("gemini-mcp.config.json"))
}

fn load_additional_args_from_config() -> Vec<String> {
    let mut base: Vec<String> = Vec::new();
    let Some(config_path) = resolve_config_path() else {
        return base;
    };

    if !config_path.is_file() {
        return base;
    }

    match std::fs::read_to_string(&config_path) {
        Ok(raw) => match serde_json::from_str::<ServerConfig>(&raw) {
            Ok(parsed) => {
                let cleaned = parsed
                    .additional_args
                    .into_iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>();

                if !cleaned.is_empty() {
                    base = cleaned;
                }
            }
            Err(err) => eprintln!(
                "gemini-mcp-rs: failed to parse config {}: {}",
                config_path.display(),
                err
            ),
        },
        Err(err) => eprintln!(
            "gemini-mcp-rs: failed to read config {}: {}",
            config_path.display(),
            err
        ),
    }

    base
}

pub fn default_additional_args() -> Vec<String> {
    static CACHED_ARGS: OnceLock<Vec<String>> = OnceLock::new();
    CACHED_ARGS
        .get_or_init(load_additional_args_from_config)
        .clone()
}

fn load_server_config() -> ServerConfig {
    let mut cfg = ServerConfig {
        additional_args: Vec::new(),
        timeout_secs: None,
    };

    let Some(config_path) = resolve_config_path() else {
        return cfg;
    };

    if !config_path.is_file() {
        return cfg;
    }

    match std::fs::read_to_string(&config_path) {
        Ok(raw) => match serde_json::from_str::<ServerConfig>(&raw) {
            Ok(parsed) => {
                let mut cleaned = parsed;
                cleaned.additional_args = cleaned
                    .additional_args
                    .into_iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                cfg = cleaned;
            }
            Err(err) => eprintln!(
                "gemini-mcp-rs: failed to parse config {}: {}",
                config_path.display(),
                err
            ),
        },
        Err(err) => eprintln!(
            "gemini-mcp-rs: failed to read config {}: {}",
            config_path.display(),
            err
        ),
    }

    cfg
}

fn server_config() -> &'static ServerConfig {
    static SERVER_CONFIG: OnceLock<ServerConfig> = OnceLock::new();
    SERVER_CONFIG.get_or_init(load_server_config)
}

pub fn default_timeout_secs() -> u64 {
    static CACHED_TIMEOUT: OnceLock<u64> = OnceLock::new();
    *CACHED_TIMEOUT.get_or_init(|| {
        let cfg = server_config();
        match cfg.timeout_secs {
            Some(t) if t > 0 && t <= MAX_TIMEOUT_SECS => t,
            Some(t) if t > MAX_TIMEOUT_SECS => MAX_TIMEOUT_SECS,
            _ => DEFAULT_TIMEOUT_SECS,
        }
    })
}

#[derive(Debug, Clone)]
pub struct Options {
    pub prompt: String,
    pub session_id: Option<String>,
    pub additional_args: Vec<String>,
}

#[derive(Debug)]
pub struct GeminiResult {
    pub success: bool,
    pub session_id: String,
    pub agent_messages: String,
    pub all_messages: Vec<Value>,
    pub error: Option<String>,
}

/// Attempt to read GEMINI.md configuration file from the current directory
/// Returns the content if found and readable, otherwise returns None
/// Logs warnings for read errors (except file not found) and size limit violations
async fn read_gemini_config() -> Option<String> {
    read_gemini_config_from_path(&PathBuf::from(GEMINI_CONFIG_FILE)).await
}

/// Internal function to read GEMINI.md configuration from a specific path
/// This is separated to allow for testing with custom paths
/// Exposed publicly for integration tests
pub async fn read_gemini_config_from_path(config_path: &PathBuf) -> Option<String> {
    // First check if file exists and get metadata
    let metadata = match fs::metadata(config_path).await {
        Ok(meta) => meta,
        Err(e) => {
            // Only log if it's not a "file not found" error
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("Warning: Cannot access GEMINI.md configuration file: {}", e);
            }
            return None;
        }
    };

    // Check file size before reading
    let file_size = metadata.len() as usize;
    if file_size > MAX_CONFIG_SIZE {
        eprintln!(
            "Warning: GEMINI.md file is too large ({} bytes, max {} bytes). Configuration will be ignored.",
            file_size, MAX_CONFIG_SIZE
        );
        return None;
    }

    // Read the file content
    match fs::read_to_string(config_path).await {
        Ok(content) => {
            // Check if content is effectively empty (only whitespace)
            if content.trim().is_empty() {
                eprintln!("Warning: GEMINI.md file is empty and will be ignored.");
                None
            } else {
                // Return original content to preserve formatting, not trimmed version
                Some(content)
            }
        }
        Err(e) => {
            eprintln!(
                "Warning: Failed to read GEMINI.md configuration file: {}",
                e
            );
            None
        }
    }
}

/// Prepare the final prompt by prepending GEMINI.md content if it exists
async fn prepare_prompt(user_prompt: &str) -> String {
    match read_gemini_config().await {
        Some(config_content) => {
            format!("{}\n\n{}", config_content, user_prompt)
        }
        None => user_prompt.to_string(),
    }
}

/// Process a single JSON line from the gemini CLI output
fn process_json_line(line_data: &Value, result: &mut GeminiResult) {
    // Collect all messages - store the raw Value to handle objects, arrays, and primitives.
    // Limit the number of messages to prevent memory exhaustion.
    if result.all_messages.len() < MAX_MESSAGES_LIMIT {
        result.all_messages.push(line_data.clone());
    }

    // Extract session_id
    if let Some(session_id) = line_data.get(KEY_SESSION_ID).and_then(|v| v.as_str()) {
        if !session_id.is_empty() {
            result.session_id = session_id.to_string();
        }
    }

    // Extract agent messages
    let item_type = line_data
        .get(KEY_TYPE)
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let item_role = line_data
        .get(KEY_ROLE)
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if item_type == TYPE_MESSAGE && item_role == ROLE_ASSISTANT {
        if let Some(content) = line_data.get(KEY_CONTENT).and_then(|v| v.as_str()) {
            // Skip the CLI's own deprecation warning about --prompt
            if content.contains(PROMPT_DEPRECATION_WARNING) {
                return;
            }
            if !result.agent_messages.is_empty() {
                result.agent_messages.push('\n');
            }
            result.agent_messages.push_str(content);
        }
    }

    // Check for errors (case-insensitive) - look for explicit error indicators
    let item_type_lower = item_type.to_lowercase();
    let has_explicit_error = item_type_lower.contains("fail") || item_type_lower.contains("error");
    let has_error_obj = line_data.get(KEY_ERROR).is_some();

    if has_explicit_error || has_error_obj {
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
    // Always stream JSON output
    cmd.arg("-o");
    cmd.arg("stream-json");

    // Additional arguments configured at the server level
    for arg in &opts.additional_args {
        cmd.arg(arg);
    }

    // Resume session if provided; otherwise, pass the prompt positionally.
    if let Some(ref session_id) = opts.session_id {
        // For resume, Gemini CLI currently requires a prompt via --prompt (-p) or stdin.
        // We use --prompt here (and filter out the deprecation warning in process_json_line).
        cmd.arg("--prompt");
        cmd.arg(&opts.prompt);
        cmd.args(["--resume", session_id]);
    } else {
        // Command::arg() on all platforms already does correct shell quoting,
        // so we pass the prompt as a positional argument without manual escaping
        cmd.arg(&opts.prompt);
    }

    cmd
}

/// Execute Gemini CLI with the given options and return the result
pub async fn run(opts: Options) -> Result<GeminiResult> {
    // Validate options
    if opts.prompt.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "Prompt must be a non-empty, non-whitespace string"
        ));
    }

    let timeout_duration = Duration::from_secs(default_timeout_secs());

    // Prepare the final prompt by prepending GEMINI.md content if it exists
    let final_prompt = prepare_prompt(&opts.prompt).await;

    // Create modified options with the final prompt
    let modified_opts = Options {
        prompt: final_prompt,
        session_id: opts.session_id,
        additional_args: opts.additional_args,
    };

    // Build the command and configure stdio.
    let mut cmd = build_command(&modified_opts);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.kill_on_drop(true);
    let mut child = cmd.spawn().context("Failed to spawn gemini command")?;

    match timeout(timeout_duration, run_with_child(&mut child)).await {
        Ok(result) => result,
        Err(_) => {
            // Explicitly kill the child process on timeout to avoid zombies
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(anyhow::anyhow!(
                "Gemini command timed out after {} seconds",
                timeout_duration.as_secs()
            ))
        }
    }
}

/// Inner function that reads from a spawned child process
async fn run_with_child(child: &mut tokio::process::Child) -> Result<GeminiResult> {
    // Read stdout and stderr
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
    let mut stderr_truncated = false;
    let mut non_json_lines = Vec::with_capacity(100); // Start with reasonable capacity
    let mut valid_json_seen = false;
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
                            Ok(data) => {
                                valid_json_seen = true;
                                data
                            }
                            Err(_) => {
                                // Collect non-JSON lines for potential logging (with limit)
                                if non_json_lines.len() < MAX_NON_JSON_LINES {
                                    non_json_lines.push(trimmed.to_string());
                                }
                                continue;
                            }
                        };

                        // Process the parsed JSON line
                        process_json_line(&line_data, &mut result);
                    }
                    None => stdout_closed = true,
                }
            }
            line = stderr_reader.next_line(), if !stderr_closed => {
                match line {
                    Ok(Some(line)) => {
                        // Only capture stderr up to the limit
                        if stderr_output.len() < MAX_STDERR_BYTES && !stderr_truncated {
                            if !stderr_output.is_empty() {
                                stderr_output.push('\n');
                            }
                            let remaining = MAX_STDERR_BYTES - stderr_output.len();
                            if line.len() <= remaining {
                                stderr_output.push_str(&line);
                            } else {
                                stderr_output.push_str(&line[..remaining]);
                                stderr_output.push_str("\n... (stderr truncated)");
                                stderr_truncated = true;
                            }
                        }
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

        let mut full_error = error_msg;
        if !stderr_output.is_empty() {
            full_error = format!("{}\nStderr: {}", full_error, stderr_output);
        }
        // Always include non-JSON output on failure to help with diagnosis
        if !non_json_lines.is_empty() {
            full_error = format!(
                "{}\nNon-JSON output: {}",
                full_error,
                non_json_lines.join("\n")
            );
        }
        result.error = Some(full_error);
    } else if !non_json_lines.is_empty() && !valid_json_seen {
        // Process succeeded but no valid JSON was seen
        result.success = false;
        result.error = Some(format!(
            "No valid JSON output received from gemini CLI.\nOutput: {}",
            non_json_lines.join("\n")
        ));
    }

    Ok(enforce_required_fields(result))
}

fn enforce_required_fields(mut result: GeminiResult) -> GeminiResult {
    let mut errors = Vec::new();

    if result.session_id.is_empty() {
        errors.push("Failed to get `SESSION_ID` from the gemini session.".to_string());
    }

    if result.agent_messages.is_empty() {
        errors.push(
            "Failed to get `agent_messages` from the gemini session.".to_string(),
        );
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
            session_id: None,
            additional_args: Vec::new(),
        };

        assert_eq!(opts.prompt, "test prompt");
    }

    #[test]
    fn test_options_with_session() {
        let opts = Options {
            prompt: "resume task".to_string(),
            session_id: Some("test-session-123".to_string()),
            additional_args: vec!["--model".to_string(), "gemini-pro".to_string()],
        };

        assert_eq!(opts.session_id, Some("test-session-123".to_string()));
        assert_eq!(opts.additional_args.len(), 2);
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
    fn test_enforce_required_fields_requires_agent_messages_when_not_returning_all() {
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
    fn test_enforce_required_fields_allows_empty_agent_messages_with_all_messages() {
        let result = GeminiResult {
            success: true,
            session_id: "session".to_string(),
            agent_messages: String::new(),
            all_messages: vec![serde_json::json!({"type": "tool_use"})],
            error: None,
        };

        let updated = enforce_required_fields(result);

        assert!(!updated.success);
        assert!(updated.error.is_some());
    }

    #[test]
    fn test_build_command_basic() {
        let opts = Options {
            prompt: "test prompt".to_string(),
            session_id: None,
            additional_args: Vec::new(),
        };

        let cmd = build_command(&opts);
        let program = cmd.as_std().get_program();

        // Should use "gemini" as the binary name (or GEMINI_BIN env var)
        assert!(program == "gemini" || program.to_string_lossy().contains("gemini"));
    }

    #[test]
    fn test_build_command_with_all_options() {
        let opts = Options {
            prompt: "complex prompt".to_string(),
            session_id: Some("session-123".to_string()),
            additional_args: vec!["--model".to_string(), "gemini-pro".to_string()],
        };

        let cmd = build_command(&opts);
        let program = cmd.as_std().get_program();

        // Should use "gemini" as the binary name
        assert!(program == "gemini" || program.to_string_lossy().contains("gemini"));
    }

    #[test]
    fn test_build_command_with_session_only() {
        let opts = Options {
            prompt: "resume".to_string(),
            session_id: Some("abc-123".to_string()),
            additional_args: Vec::new(),
        };

        let cmd = build_command(&opts);
        let program = cmd.as_std().get_program();

        assert!(program == "gemini" || program.to_string_lossy().contains("gemini"));
    }

    #[test]
    fn test_process_json_line_skips_prompt_deprecation_warning() {
        let mut result = GeminiResult {
            success: true,
            session_id: String::new(),
            agent_messages: String::new(),
            all_messages: Vec::new(),
            error: None,
        };

        let line = serde_json::json!({
            "session_id": "test-session",
            "type": "message",
            "role": "assistant",
            "content": format!(
                "{} and will be removed in a future version. Please use a positional argument.",
                PROMPT_DEPRECATION_WARNING
            )
        });

        process_json_line(&line, &mut result);

        // Warning should not be treated as an agent message, but session_id should still be set.
        assert_eq!(result.session_id, "test-session");
        assert!(result.agent_messages.is_empty());
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_read_gemini_config_nonexistent_file() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("GEMINI.md");

        let result = read_gemini_config_from_path(&config_path).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_read_gemini_config_with_content() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("GEMINI.md");

        let test_content = "Test configuration content";
        fs::write(&config_path, test_content).await.unwrap();

        let result = read_gemini_config_from_path(&config_path).await;
        assert_eq!(result, Some(test_content.to_string()));
    }

    #[tokio::test]
    async fn test_read_gemini_config_empty_file() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("GEMINI.md");

        // File with only whitespace should be considered empty
        fs::write(&config_path, "   \n  \n  ").await.unwrap();

        let result = read_gemini_config_from_path(&config_path).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_read_gemini_config_preserves_formatting() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("GEMINI.md");

        // Content with intentional leading/trailing whitespace and newlines
        let test_content = "\n# Header\n\nContent with spaces.  \n\n";
        fs::write(&config_path, test_content).await.unwrap();

        let result = read_gemini_config_from_path(&config_path).await;
        // Should preserve original formatting, not trim it
        assert_eq!(result, Some(test_content.to_string()));
    }

    #[tokio::test]
    async fn test_read_gemini_config_too_large() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("GEMINI.md");

        let large_content = "x".repeat(MAX_CONFIG_SIZE + 1);
        fs::write(&config_path, large_content).await.unwrap();

        let result = read_gemini_config_from_path(&config_path).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_prepare_prompt_without_config() {
        use tempfile::TempDir;
        let _temp_dir = TempDir::new().unwrap();

        let user_prompt = "Test user prompt";
        let result = prepare_prompt(user_prompt).await;

        // Without config, prompt should be unchanged
        assert!(result.contains(user_prompt));
    }

    #[tokio::test]
    async fn test_prepare_prompt_preserves_user_prompt() {
        let user_prompt = "What is 2+2?";
        let result = prepare_prompt(user_prompt).await;

        assert!(result.contains(user_prompt));
    }
}

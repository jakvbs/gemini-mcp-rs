// Tests focused on verifying that additional_args are correctly
// passed through to the Gemini CLI.

use gemini_mcp_rs::gemini;
use gemini_mcp_rs::gemini::Options;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[tokio::test]
async fn test_additional_args_are_passed_to_gemini_cli() {
    let temp_path = env::temp_dir();

    // Path where the helper script will log its argv
    let log_path = temp_path.join("gemini_args.log");

    // Create a helper script that logs argv and emits a minimal JSON event
    let script_path = temp_path.join("echo_args.sh");
    let script_contents = r#"#!/bin/sh
LOG_FILE="${GEMINI_ARGS_LOG}"
: > "$LOG_FILE"
printf "%s" "$0" > "$LOG_FILE"
for arg in "$@"; do
  printf " %s" "$arg" >> "$LOG_FILE"
done
echo '{"session_id":"test-session","type":"message","role":"assistant","content":"ok"}'
"#;

    fs::write(&script_path, script_contents).expect("Failed to write script");
    let mut perms = fs::metadata(&script_path)
        .expect("Failed to get metadata")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("Failed to set permissions");

    env::set_var("GEMINI_BIN", script_path.to_str().unwrap());

    // Make log path available to the helper script
    env::set_var("GEMINI_ARGS_LOG", log_path.to_str().unwrap());

    let additional = vec!["--model".to_string(), "gemini-3-pro-preview".to_string()];

    let opts = Options {
        prompt: "test additional args".to_string(),
        session_id: None,
        additional_args: additional.clone(),
    };

    let result = gemini::run(opts).await.expect("run should return Ok");

    assert!(result.success, "helper script should succeed");
    assert_eq!(result.session_id, "test-session");
    assert_eq!(result.agent_messages.trim(), "ok");

    // Verify that additional_args were passed through to the Gemini CLI
    let log = fs::read_to_string(&log_path).expect("failed to read args log");
    let parts: Vec<&str> = log.split_whitespace().collect();

    let idx = parts
        .iter()
        .position(|s| *s == "--model")
        .expect("additional_args flag not found in argv");

    assert_eq!(parts.get(idx + 1), Some(&"gemini-3-pro-preview"));

    // Clean up env vars
    env::remove_var("GEMINI_BIN");
    env::remove_var("GEMINI_ARGS_LOG");
}

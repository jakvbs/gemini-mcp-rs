// Common test utilities and helpers

/// Get a temporary directory for testing
pub fn get_temp_dir() -> std::path::PathBuf {
    std::env::temp_dir()
}

/// Create a test options with default values
pub fn create_test_options(prompt: &str) -> gemini_mcp_rs::gemini::Options {
    gemini_mcp_rs::gemini::Options {
        prompt: prompt.to_string(),
        session_id: None,
        additional_args: Vec::new(),
    }
}

/// Mock session ID generator
pub fn generate_mock_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("test-session-{}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_temp_dir() {
        let temp = get_temp_dir();
        assert!(temp.exists());
        assert!(temp.is_dir());
    }

    #[test]
    fn test_create_test_options() {
        let opts = create_test_options("test prompt");
        assert_eq!(opts.prompt, "test prompt");
    }

    #[test]
    fn test_generate_mock_session_id() {
        let id1 = generate_mock_session_id();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let id2 = generate_mock_session_id();

        assert!(id1.starts_with("test-session-"));
        assert!(id2.starts_with("test-session-"));
        assert_ne!(id1, id2);
    }
}


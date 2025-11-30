// Integration tests for gemini-mcp-rs
// These tests require a real Gemini CLI installation or mock

#[cfg(test)]
mod tests {
    use gemini_mcp_rs::gemini::Options;

    #[tokio::test]
    #[ignore] // Ignore by default - requires Gemini CLI
    async fn test_gemini_execution() {
        let _opts = Options {
            prompt: "Hello, world!".to_string(),
            session_id: None,
            additional_args: Vec::new(),
        };

        // This test requires a real Gemini CLI installation
        // Uncomment and set GEMINI_BIN environment variable to run
        // let result = gemini_mcp_rs::gemini::run(opts).await;
        // assert!(result.is_ok());
    }

    #[test]
    fn test_options_validation() {
        let opts = Options {
            prompt: "test".to_string(),
            session_id: Some("session-123".to_string()),
            additional_args: vec!["--model".to_string(), "gemini-pro".to_string()],
        };

        assert_eq!(opts.prompt, "test");
        assert_eq!(opts.session_id, Some("session-123".to_string()));
        assert_eq!(opts.additional_args.len(), 2);
    }
}

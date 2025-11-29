use gemini_mcp_rs::server::GeminiServer;
use rmcp::{model::*, ServerHandler};

#[test]
fn test_server_creation() {
    let _server = GeminiServer::new();
    // Test passes if server creation doesn't panic
}

#[test]
fn test_server_info() {
    let server = GeminiServer::new();
    let info = server.get_info();

    assert_eq!(info.protocol_version, ProtocolVersion::V_2024_11_05);
    assert!(info.capabilities.tools.is_some());
    assert!(info.instructions.is_some());
}

#[test]
fn test_default_implementation() {
    let server1 = GeminiServer::new();
    let server2 = GeminiServer::default();

    // Both should create valid servers
    let info1 = server1.get_info();
    let info2 = server2.get_info();

    assert_eq!(info1.protocol_version, info2.protocol_version);
}

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial Rust port from Python implementation
- MCP server implementation using official Rust SDK (rmcp)
- Gemini CLI wrapper with JSON output parsing
- Session management for multi-turn conversations
- Sandbox mode support
- Async I/O with Tokio runtime
- Cross-platform support (Linux, macOS, Windows)
- Comprehensive documentation

## [0.1.0] - 2025-01-28

### Added
- Initial release of gemini-mcp-rs
- MCP server implementation using official Rust SDK (rmcp)
- Gemini CLI wrapper with JSON output parsing
- Session management for multi-turn conversations
- Configurable sandbox mode
- Async I/O with Tokio runtime
- NPM package with automatic binary downloads
- Cross-platform support (Linux, macOS, Windows × x86_64, arm64)
- GitHub Actions CI/CD workflows
- Comprehensive documentation (README, CLAUDE.md, CONTRIBUTING.md, QUICKSTART.md)
- MIT License

### Features
- **Tool**: `gemini` - Invokes the Gemini CLI to execute AI-driven tasks
  - Required parameters: `PROMPT`
  - Optional parameters: `sandbox`, `SESSION_ID`, `return_all_messages`, `model`
- **Transport**: stdio (standard input/output)
- **Error handling**: Comprehensive validation and error messages
- **Performance**: High-performance Rust implementation with low memory footprint

### Documentation
- Installation guides (npm, binary, source)
- Usage examples and common use cases
- Architecture documentation for developers
- Contribution guidelines
- Quick start guide

### Infrastructure
- Automated multi-platform builds
- NPM package publishing
- MCP registry integration
- Continuous Integration testing
- Makefile for development convenience

[Unreleased]: https://github.com/missdeer/gemini-mcp-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/missdeer/gemini-mcp-rs/releases/tag/v0.1.0


# gemini-mcp-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io)

A high-performance Rust implementation of MCP (Model Context Protocol) server that wraps the Gemini CLI for AI-driven tasks.

> **Note**: This is a Rust port of the original Python implementation [geminimcp](../geminimcp). It offers the same functionality with improved performance and lower resource usage.

## Features

- **MCP Protocol Support**: Implements the official Model Context Protocol using the Rust SDK
- **Gemini Integration**: Wraps the Gemini CLI to enable AI-driven tasks through MCP
- **Session Management**: Supports multi-turn conversations via session IDs
- **Sandbox Safety**: Configurable sandbox mode for isolated execution
- **Async Runtime**: Built on Tokio for efficient async I/O
- **Cross-platform**: Works on Windows, Linux, and macOS

## Prerequisites

- Rust 1.90+ (uses 2021 edition)
- [Gemini CLI](https://github.com/google-gemini/gemini-cli) installed and configured
- Claude Code or another MCP client

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

## Running

The server communicates via stdio transport:

```bash
cargo run
```

Or after building:

```bash
./target/release/gemini-mcp-rs
```

### Command-Line Options

```bash
# Display help information
./target/release/gemini-mcp-rs --help

# Display version information
./target/release/gemini-mcp-rs --version
```

The `--help` flag provides comprehensive documentation including:
- Environment variables
- MCP client configuration examples
- All supported tool parameters
- GEMINI.md configuration file support
- Return structure format
- Best practices and security information

## Installation

### Option 1: Quick Install (Linux/macOS)

Install the latest release with a single command:

```bash
curl -sSL https://raw.githubusercontent.com/missdeer/gemini-mcp-rs/master/scripts/install.sh | bash
```

Or install a specific version:

```bash
curl -sSL https://raw.githubusercontent.com/missdeer/gemini-mcp-rs/master/scripts/install.sh | bash -s v0.1.0
```

This script will:
- Detect your platform and architecture
- Download the appropriate binary from GitHub releases
- Install it to `~/.local/bin` (or `/usr/local/bin` if needed)
- Automatically add it to your Claude MCP configuration

### Option 2: Build from Source

```bash
git clone https://github.com/missdeer/gemini-mcp-rs.git
cd gemini-mcp-rs
cargo build --release
claude mcp add gemini-rs -s user --transport stdio -- $(pwd)/target/release/gemini-mcp-rs
```

### Option 3: Install from Release

Download the appropriate binary for your platform from the releases page, extract it, and add to your MCP configuration:

```bash
claude mcp add gemini-rs -s user --transport stdio -- /path/to/gemini-mcp-rs
```

## Tool Usage

The server provides a single `gemini` tool with the following parameters:

### Required Parameters

- `PROMPT` (string): Instruction for the task to send to gemini

### Optional Parameters

- `SESSION_ID` (string): Resume a previously started Gemini session. Use exactly
  the `SESSION_ID` value returned from an earlier `gemini` tool call (typically
  a UUID like `89473362-3f12-46e8-adce-05388980dcca`). If omitted, a new session
  is created. Custom labels (for example `"skinbase-tradeit-metrics"`) are not
  valid session identifiers. Never send an empty string value: when starting a
  new session, omit the `SESSION_ID` field entirely instead of passing `""`.

### Return Structure

**Success:**
```json
{
  "success": true,
  "SESSION_ID": "session-uuid",
  "message": "Gemini's reply content..."
}
```

**Failure:**
```json
{
  "success": false,
  "error": "Error description"
}
```

## Best Practices

- Always capture and reuse `SESSION_ID` for multi-turn interactions
- Configure Gemini CLI flags such as sandbox mode or model selection at the CLI/config level rather than as tool parameters

## Configuration

### Environment Variables

- `GEMINI_BIN`: Override the Gemini CLI binary path. By default, the server uses `gemini` from your PATH. This is useful for:
  - Using a specific Gemini installation location
  - Testing with a custom binary
  - Development environments with multiple Gemini versions

  **Example:**
  ```bash
  export GEMINI_BIN=/usr/local/bin/gemini-custom
  cargo run
  ```

### JSON Configuration

The server can load additional Gemini CLI arguments and a default timeout from a JSON configuration file. By default it looks for `gemini-mcp.config.json` in the current working directory, or a custom path specified via `GEMINI_MCP_CONFIG_PATH`.

Example:

```json
{
  "additional_args": [
    "--model",
    "gemini-3-pro-preview"
  ],
  "timeout_secs": 600
}
```

These `additional_args` are appended to every Gemini CLI invocation after the core flags (`-o stream-json`) and before any `--resume` session flag. The optional `timeout_secs` controls the maximum runtime for each Gemini execution (default 600 seconds, capped at 3600 when set higher).

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Test with a custom Gemini binary
GEMINI_BIN=/path/to/gemini cargo test
```

## Architecture

The project follows a modular architecture:

- `src/main.rs`: Entry point that parses CLI arguments and starts the MCP server
- `src/lib.rs`: Library root that exports modules
- `src/server.rs`: MCP server implementation and tool handlers
- `src/gemini.rs`: Gemini CLI execution and result parsing

## Comparison with Python Implementation

| Feature | gemini-mcp-rs (Rust) | geminimcp (Python) |
|---------|---------------------|-------------------|
| Language | Rust | Python |
| Performance | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| Memory Usage | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| Binary Size | Medium | N/A |
| Startup Time | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| Session Management | ✓ | ✓ |
| Sandbox Support | ✓ | ✓ |

## Related Projects

- [geminimcp](https://github.com/GuDaStudio/geminimcp) - Original Python implementation
- [codex-mcp-rs](https://github.com/missdeer/codex-mcp-rs) - Rust MCP server for Codex CLI

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - Copyright (c) 2025 missdeer

See [LICENSE](./LICENSE) for details.


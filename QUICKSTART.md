# Quick Start Guide

Get started with gemini-mcp-rs in 5 minutes!

## Prerequisites

1. Install [Gemini CLI](https://github.com/google-gemini/gemini-cli):
   ```bash
   # Follow Gemini CLI installation instructions
   gemini --version
   ```

2. Install [Claude Code](https://docs.claude.com/docs/claude-code):
   ```bash
   # Follow Claude Code installation instructions
   claude --version
   ```

## Installation

### Using NPM (Recommended)

```bash
# Install globally
npm install -g @missdeer/gemini-mcp-rs

# Add to Claude Code
claude mcp add gemini-rs -s user --transport stdio -- gemini-mcp-rs
```

### Using Pre-built Binary

1. Download from [releases](https://github.com/missdeer/gemini-mcp-rs/releases)
2. Extract the archive
3. Add to Claude Code:
   ```bash
   claude mcp add gemini-rs -s user --transport stdio -- /path/to/gemini-mcp-rs
   ```

### Building from Source

```bash
# Clone repository
git clone https://github.com/missdeer/gemini-mcp-rs.git
cd gemini-mcp-rs

# Build release binary
cargo build --release

# Add to Claude Code
claude mcp add gemini-rs -s user --transport stdio -- $(pwd)/target/release/gemini-mcp-rs
```

## Verification

Check that the server is registered:

```bash
claude mcp list
```

You should see:
```
gemini-rs: gemini-mcp-rs - ✓ Connected
```

## Basic Usage

In Claude Code, you can now use the `gemini` tool:

```
Use the gemini tool to help design a beautiful frontend interface
```

Claude Code will call the gemini tool with:
```json
{
  "PROMPT": "help design a beautiful frontend interface"
}
```

## Common Use Cases

### 1. Frontend Design

```
Use gemini to create a modern, responsive landing page design
```

### 2. UI Components

```
Use gemini to design a card component with hover effects
```

### 3. Multi-turn Conversation

```
First call:
Use gemini to analyze the design requirements

Second call (using SESSION_ID from first response):
Now refine the color scheme and typography
SESSION_ID: <previous-session-id>
```

## Configuration

### Sandbox Mode

Enable sandbox mode for isolated execution:
```json
{
  "PROMPT": "design a component",
  "sandbox": true
}
```

### Return All Messages

Get detailed execution traces:
```json
{
  "PROMPT": "design a component",
  "return_all_messages": true
}
```

## Troubleshooting

### "command not found: gemini-mcp-rs"

NPM binary not in PATH. Try:
```bash
npm list -g @missdeer/gemini-mcp-rs
which gemini-mcp-rs
```

If installed, add npm global bin to PATH:
```bash
export PATH="$PATH:$(npm bin -g)"
```

### "Failed to execute gemini"

Check Gemini CLI is installed:
```bash
gemini --version
```

### Server won't start

Check logs:
```bash
claude mcp logs gemini-rs
```

## Next Steps

- Read [README.md](./README.md) for detailed features
- See [CLAUDE.md](./CLAUDE.md) for architecture details
- Check [CONTRIBUTING.md](./CONTRIBUTING.md) to contribute
- Browse examples for code samples

## Getting Help

- 🐛 [Report bugs](https://github.com/missdeer/gemini-mcp-rs/issues)
- 💬 [Discussions](https://github.com/missdeer/gemini-mcp-rs/discussions)


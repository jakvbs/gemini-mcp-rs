#!/bin/bash

# Get the absolute path of the gemini-mcp-rs binary
# if current os is Darwin, use $(pwd)/gemini-mcp-rs
if [ "$(uname)" == "Darwin" ]; then
    GEMINI_MCP_RS_PATH=$(pwd)/gemini-mcp-rs
fi
if [ ! -f "$GEMINI_MCP_RS_PATH" ]; then
    GEMINI_MCP_RS_PATH=$(pwd)/target/release/gemini-mcp-rs
    if [ ! -f "$GEMINI_MCP_RS_PATH" ]; then
        echo "Error: gemini-mcp-rs binary not found"
        exit 1
    fi
fi

# Add the gemini-mcp-rs server to the Claude Code MCP registry
CLAUDE_PATH=$(which claude)
if [ -f "$CLAUDE_PATH" ]; then
    "$CLAUDE_PATH" mcp add gemini-rs -s user --transport stdio -- "$GEMINI_MCP_RS_PATH"
else
    echo "Error: claude not found"
    exit 1
fi
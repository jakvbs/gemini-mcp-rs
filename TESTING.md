# Testing Guide

This document describes the testing strategy and how to run tests for gemini-mcp-rs.

## Current Status

✅ **Unit tests** for core functionality
✅ **Clippy clean** - no warnings
✅ **CI integration** - automated testing on multiple platforms

## Test Structure

```
gemini-mcp-rs/
├── src/
│   ├── gemini.rs         # Contains unit tests for Options validation
│   ├── server.rs        # Contains unit tests for server implementation
│   └── main.rs
└── tests/
    ├── common/
    │   └── mod.rs       # Shared test utilities
    ├── integration_tests.rs  # Integration tests
    └── server_tests.rs       # Server-specific tests
```

## Running Tests

### Run All Tests

```bash
cargo test
```

### Run Specific Test Suites

```bash
# Run only unit tests (in src/)
cargo test --lib

# Run only integration tests (in tests/)
cargo test --test '*'

# Run only doc tests
cargo test --doc

# Run tests for a specific file
cargo test --test integration_tests

# Run a specific test by name
cargo test test_options_creation
```

### Run Tests with Output

```bash
# Show println! output
cargo test -- --nocapture

# Show output and run tests one by one
cargo test -- --nocapture --test-threads=1
```

### Run Tests in Release Mode

```bash
cargo test --release
```

## Test Categories

### 1. Unit Tests (src/gemini.rs)

Tests for Options validation and result processing:

- `test_options_creation` - Options struct validation
- `test_options_with_session` - Options with session ID
- `test_enforce_required_fields_requires_session_id` - Session ID validation
- `test_enforce_required_fields_requires_agent_messages` - Agent messages validation

Run with:
```bash
cargo test --lib
```

### 2. Server Tests (src/server.rs)

Tests for the MCP server implementation:

- `test_gemini_args_deserialization` - Parameter deserialization
- `test_gemini_args_empty_session_id` - Empty session ID handling
- `test_gemini_output_serialization` - Output serialization

Run with:
```bash
cargo test --lib
```

### 3. Integration Tests (tests/integration_tests.rs)

End-to-end tests that require a real Gemini CLI installation:

- Tests actual Gemini CLI execution
- Tests session management
- Tests error handling

Run with:
```bash
cargo test --test integration_tests
```

### 4. Server Tests (tests/server_tests.rs)

Tests for MCP protocol implementation:

- Server initialization
- Server info validation
- Tool registration

Run with:
```bash
cargo test --test server_tests
```

## Writing New Tests

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        let result = my_function("input");
        assert_eq!(result, "expected");
    }
}
```

### Integration Test Example

```rust
#[tokio::test]
async fn test_gemini_execution() {
    let opts = Options {
        prompt: "test".to_string(),
        session_id: None,
        additional_args: Vec::new(),
    };
    
    let result = gemini::run(opts).await;
    assert!(result.is_ok());
}
```

## Test Coverage

Generate coverage report:

```bash
cargo tarpaulin --out Html --out Xml --all-features
```

View HTML report:
```bash
open tarpaulin-report.html
```

## Continuous Integration

Tests run automatically on:
- Every push to main branch
- Every pull request
- Multiple platforms (Ubuntu, macOS)
- Multiple Rust versions (stable, beta)

See `.github/workflows/ci.yml` for details.

## Best Practices

1. **Test edge cases**: Empty strings, None values, invalid inputs
2. **Test error paths**: Network failures, parse errors, invalid responses
3. **Keep tests fast**: Use mocks for external dependencies when possible
4. **Test in isolation**: Each test should be independent
5. **Use descriptive names**: Test names should clearly describe what they test

## Troubleshooting

### Tests fail with "command not found: gemini"

Set the `GEMINI_BIN` environment variable to point to your Gemini CLI:
```bash
export GEMINI_BIN=/path/to/gemini
cargo test
```

### Tests timeout

Some integration tests may take longer. Increase timeout:
```bash
cargo test -- --test-threads=1 --nocapture
```

### Flaky tests

Ensure tests are deterministic:
- Don't rely on timing
- Use fixed test data
- Mock external dependencies


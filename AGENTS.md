# Rustrland Agent Guidelines

## Build Commands
- **Build**: `cargo build` or `make build`
- **Release build**: `cargo build --release` or `make build-release`
- **Check compilation**: `cargo check` or `make check`

## Test Commands
- **All tests**: `cargo test --all-features --workspace` or `make test`
- **Single test**: `cargo test test_name` (replace test_name with function name)
- **Integration tests**: `cargo test --test integration_test`
- **Run specific test file**: `cargo test --test filename` (without .rs extension)

## Lint & Format
- **Lint**: `cargo clippy --lib --bins -- -D warnings` or `make lint`
- **Format**: `cargo fmt` or `make fmt`
- **Format check**: `cargo fmt --check` or `make fmt-check`

## Validation
- **Full CI check**: `make ci` (format + lint + test + release build)
- **Pre-push validation**: `./scripts/pre-push.sh`

## Code Style Guidelines

### Imports & Organization
- Group imports: std → external crates → local modules
- Use absolute paths for clarity: `use crate::module::Type`
- Avoid wildcard imports except for prelude modules

### Naming Conventions
- **Functions/variables**: snake_case
- **Types/structs/enums**: PascalCase
- **Constants**: SCREAMING_SNAKE_CASE
- **Modules**: snake_case

### Error Handling
- Use `anyhow::Result<T>` for application errors
- Use `thiserror` for library error types
- Prefer early returns with `?` operator
- Log errors with appropriate tracing levels

### Async Code
- Use `#[tokio::main]` for main functions
- Prefer `async fn` for async operations
- Use `tokio::spawn` for concurrent tasks

### Documentation
- Use `//!` for module-level documentation
- Use `///` for public API documentation
- Keep comments concise and actionable

### Testing
- Use `#[tokio::test]` for async tests
- Use descriptive test names: `test_feature_scenario`
- Test both success and error paths
- Use `tempfile` for temporary test files

### Performance
- Release profile uses LTO and symbol stripping
- Prefer owned types over references when possible
- Use `Arc<RwLock<T>>` for shared mutable state
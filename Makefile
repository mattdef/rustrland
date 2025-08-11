.PHONY: build test run dev fmt lint install clean

# Build the project
build:
	cargo build

# Build release version
build-release:
	cargo build --release

# Run tests
test:
	cargo test

# Run in development mode
run:
	cargo run --bin rustrland -- --debug --foreground

# Development with auto-reload
dev:
	cargo watch -x 'run --bin rustrland -- --debug --foreground'

# Format code
fmt:
	cargo fmt

# Lint code
lint:
	cargo clippy -- -D warnings

# Install locally
install:
	cargo install --path .

# Clean build artifacts
clean:
	cargo clean

# Check if everything compiles
check:
	cargo check

# Run with example config
run-example:
	cargo run --bin rustrland -- --config examples/rustrland.toml --debug --foreground

# Full CI check
ci: fmt lint test build

# Help
help:
	@echo "Available targets:"
	@echo "  build         - Build the project"
	@echo "  build-release - Build release version"
	@echo "  test          - Run tests"
	@echo "  run           - Run in development mode"
	@echo "  dev           - Development with auto-reload"
	@echo "  fmt           - Format code"
	@echo "  lint          - Lint code"
	@echo "  install       - Install locally"
	@echo "  clean         - Clean build artifacts"
	@echo "  check         - Check compilation"
	@echo "  run-example   - Run with example config"
	@echo "  ci            - Full CI check"

# Elysium Engine Commands

## Build Commands

```bash
# Build entire project
cargo build

# Build specific package
cargo build --package elysium
cargo build --package elysium-core
cargo build --package elysium-render
cargo build --package elysium-physics
cargo build --package elysium-ui
cargo build --package elysium-asset
cargo build --package elysium-editor
cargo build --package elysium-net
cargo build --package elysium-flux

# Build in release mode
cargo build --release
```

## Run Commands

```bash
# Run main application
cargo run --package elysium

# Run with release optimizations
cargo run --package elysium --release
```

## Check Commands

```bash
# Run clippy linter
cargo clippy --all-targets --all-features

# Check for compilation errors
cargo check --all-targets
```

## Test Commands

```bash
# Run all tests
cargo test

# Run tests for specific package
cargo test --package elysium-core
```

## Clean Commands

```bash
# Clean build artifacts
cargo clean
```
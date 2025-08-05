# README.md - Development Guidelines

## Build Commands
- Build: `cargo build`
- Run: `cargo run`
- Release build: `cargo build --release`

## Test Commands
- Run all tests: `cargo test`
- Run specific test: `cargo test test_name`
- Run tests with output: `cargo test -- --nocapture`

## Lint/Format Commands
- Format code: `cargo fmt`
- Check formatting: `cargo fmt --check`
- Lint: `cargo clippy`
- Lint with fixes: `cargo clippy --fix`

## Code Style Guidelines

### Imports
- Group imports in order: standard library, external crates, local modules
- Use `use` statements at the top of the file
- Avoid glob imports (`*`) except for prelude modules

### Formatting
- Use `cargo fmt` for consistent formatting
- Line width: 100 characters
- Indentation: 4 spaces

### Types
- Prefer explicit types for public API
- Use `Option<T>` for values that might be absent
- Use `Result<T, E>` for operations that might fail

### Naming Conventions
- Variables: snake_case
- Functions: snake_case
- Structs/Enums: PascalCase
- Constants: UPPER_SNAKE_CASE
- Modules: snake_case

### Error Handling
- Use `Result<T, Box<dyn Error>>` for main functions
- Prefer `?` operator for error propagation
- Use `eprintln!` for error logging
- Handle errors gracefully rather than panicking

### Additional Notes
- This is a TUI music player using ratatui and rodio
- Audio processing happens in separate threads
- UI updates happen in the main thread
- Use Arc<Mutex<T>> for shared state between threads

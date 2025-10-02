# README.md

## Goal

This is a simple CLI music player using ratatui and rodio. Audio processing happens in separate threads, UI updates happen in the main thread, and Arc<Mutex<T>> are used for shared state between threads.

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

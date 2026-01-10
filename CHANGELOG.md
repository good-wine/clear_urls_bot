# Changelog

All notable changes to this project will be documented in this file.

## [1.1.0] - 2026-01-10

### Added
- **Smart Language Detection**: Automatically detects the language of incoming messages (English/Italian) and replies in the corresponding language.
- **Supabase Integration**: Compatibility with Supabase PostgreSQL for persistent cloud storage.
- **WASM URL Cleaner**: High-performance Rust-compiled WebAssembly module for client-side sanitization.
- **Advanced Observability**: Robust logging system using `tracing` with JSON output support for production and colored pretty-logs for development.
- **Multi-Database Support**: Implemented dynamic backend detection (SQLite/Postgres) using `sqlx::Any`.

### Changed
- Refactored project structure into a modular library (`src/lib.rs`) and binary (`src/main.rs`).
- Upgraded all dependencies to latest major versions (`teloxide 0.17`, `axum 0.8`, `sqlx 0.8`).
- Improved documentation with detailed architecture and observability guides.
- Hardened web dashboard with Axum 0.8 compatibility and enhanced route safety.

### Fixed
- Deprecated `teloxide` method calls and updated to new `reply_parameters` API.
- Fixed `reqwest` TLS feature naming conflicts in version 0.13.
- **Zero-Panic Core**: Eliminated all `unwrap()` calls in favor of graceful error handling and descriptive status codes.
- **Bot Command Handling**: Fixed `/start` command compatibility in group chats and with bot handles.

# Architecture Overview 🏗️

This project is designed with modularity and scalability in mind, supporting multiple deployment environments from local containers to serverless clouds.

## 📦 Component Structure

### 1. Core Library (`src/lib.rs`)
The backbone of the application. It exports all core modules:
- `sanitizer`: The regex-based engine that processes URLs. Hardened against lock poisoning.
- `ai_sanitizer`: Optional deep-scan logic using LLMs.
- `db`: Database abstraction layer using **sqlx::Any**, supporting both **PostgreSQL** and **SQLite** dynamically.
- `bot`: Telegram bot handler logic (Teloxide).
- `web`: Axum-based web dashboard and API. Re-engineered for Axum 0.8 compatibility.

### 2. Standalone Binary (`src/main.rs`)
Used for Docker and local deployments. It initializes the database, starts the Telegram bot (long polling), and runs the Axum web server concurrently using `tokio::select!`.

### 3. Serverless Entry Point (`api/index.rs`)
A specialized entry point for Vercel. It is defined as a formal binary target named `index` in `Cargo.toml`. It bridges the Axum `Router` from the core library to Vercel's serverless runtime. 

### 4. WASM Module (`wasm-functions/`)
A standalone Rust crate that compiles the sanitization logic to WebAssembly, allowing for zero-latency URL cleaning in the browser.

## 🔄 Data Flow

1. **Telegram Update** -> `src/bot.rs` -> `src/sanitizer.rs` -> **Database Log**.
2. **Web Dashboard** -> `api/index.rs` (Vercel) OR `src/web.rs` (Local) -> `src/db.rs` -> **UI Render**.
3. **SSE Events** -> Real-time updates pushed from the bot task to the web dashboard (local only).

## 📊 Database Schema
The system uses SQLx with automatic migrations and dynamic backend detection.
- `user_configs`: Global settings per user.
- `chat_configs`: Specific settings per Telegram group.
- `cleaned_links`: Audit log of all sanitized URLs.
- `custom_rules`: User-defined regex patterns.

## 🛡️ Reliability & Stability
- **Zero-Panic Policy**: The codebase has been refactored to remove all `unwrap()` calls in the core logic. Errors are handled gracefully via `Result` types and meaningful HTTP status codes.
- **Dynamic Drivers**: The same binary can run against a local `.db` file or a production RDS/Supabase instance without recompilation.
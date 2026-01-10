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

...

## 📊 Database Schema
The system uses SQLx with automatic migrations and dynamic backend detection.
- `user_configs`: Global settings per user.
- `chat_configs`: Specific settings per Telegram group.
- `cleaned_links`: Audit log of all sanitized URLs.
- `custom_rules`: User-defined regex patterns.

## 🛡️ Reliability & Stability
- **Zero-Panic Policy**: The codebase has been refactored to remove all `unwrap()` calls in the core logic. Errors are handled gracefully via `Result` types and meaningful HTTP status codes.
- **Dynamic Drivers**: The same binary can run against a local `.db` file or a production RDS/Supabase instance without recompilation.

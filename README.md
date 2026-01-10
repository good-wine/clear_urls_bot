# ClearURLs Telegram Bot 🛡️

[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![WASM](https://img.shields.io/badge/WASM-supported-blueviolet.svg)](wasm-functions/)

A professional, high-performance Rust-based Telegram bot that automatically removes tracking parameters from URLs. Featuring a modular architecture and real-time web dashboard.

## 📖 Documentation

- **[Architecture Guide](docs/ARCHITECTURE.md)**: Deep dive into the modular structure and data flow.
- **[API & Dashboard](docs/API.md)**: Details on web routes and management features.
- **[Contributing](CONTRIBUTING.md)**: How to set up development and submit changes.
- **[Changelog](CHANGELOG.md)**: History of releases and updates.

## 🌟 Key Features

- **Smart Language Detection**: Automatically detects and responds in English or Italian based on message context and user settings.
- **Real-Time Dashboard**: Updates stats and history instantly via **Server-Sent Events (SSE)** without page reloads.
- **Telegram Web App Integration**: Access the full management dashboard directly inside Telegram.
- **Modern UI/UX with Dark Mode**: Responsive design that automatically adapts to system themes (Light/Dark).
- **Multi-Language Support**: Full i18n support for Italian and English.
- **Granular Control**: Per-chat configuration (Reply/Delete modes) and custom tracking parameter removal.
- **AI Deep Scan**: Optional AI-powered sanitization for complex tracking parameters not covered by standard rules.
- **Shortlink Expansion**: Automatically follows redirects from services like bit.ly or tinyurl to uncover and strip underlying trackers.
- **Deep Auditing**: Track which provider (Amazon, Google, etc.) cleaned each link.
- **CSV Export**: Download your full cleaning history for personal analysis.
- **Enterprise Ready**: Multi-stage Podman build and automatic configuration validation.

## 🛠️ Bot Commands

- `/start` - Initial setup, shows your User ID and provides access to the Web App.
- `/help` - Usage instructions and command list.
- `/stats` - View your personal cleaning statistics in-chat.

## 🚀 Quick Start

1. **Clone & Configure**:
   - Copy `.env.example` to `.env`.
   - Set `TELOXIDE_TOKEN`, `BOT_USERNAME`, `DASHBOARD_URL`, and `ADMIN_ID`.
   - **Important**: Generate a random `COOKIE_KEY` for session persistence.
   - (Optional) Set `AI_API_KEY`, `AI_API_BASE`, and `AI_MODEL` for AI Deep Scan.

3. **Run with Podman**:
   ```bash
   podman-compose up --build -d
   ```
   *Or locally with `cargo run --release`.*

## 🏗️ Technical Architecture

- **Backend**: Rust 2021, Axum 0.8 (Web), Teloxide 0.17 (Bot).
- **Database**: sqlx::Any (SQLite/PostgreSQL) with dynamic backend detection.
- **Real-time**: Async broadcast channels with SSE streaming.
- **Stability**: Zero-panic core logic with comprehensive `tracing` instrumentation.

## 📝 License

MIT

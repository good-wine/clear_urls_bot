# ClearURLs Telegram Bot 🛡️

[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![WASM](https://img.shields.io/badge/WASM-supported-blueviolet.svg)](wasm-functions/)

A professional, high-performance Rust-based Telegram bot that automatically removes tracking parameters from URLs. Designed for direct command-based interaction.

## 📖 Documentation

- **[Architecture Guide](docs/ARCHITECTURE.md)**: Deep dive into the modular structure and data flow.
- **[Contributing](CONTRIBUTING.md)**: How to set up development and submit changes.
- **[Changelog](CHANGELOG.md)**: History of releases and updates.

## 🌟 Key Features

- **Smart Language Detection**: Automatically detects and responds in English or Italian based on message context and user settings.
- **Multi-Language Support**: Full i18n support for Italian and English.
- **Granular Control**: Per-chat configuration (Reply/Delete modes) and custom tracking parameter removal.
- **AI Deep Scan**: Optional AI-powered sanitization for complex tracking parameters not covered by standard rules.
- **Shortlink Expansion**: Automatically follows redirects from services like bit.ly or tinyurl to uncover and strip underlying trackers.
- **Deep Auditing**: Track which provider (Amazon, Google, etc.) cleaned each link.
- **CSV Export**: Download your full cleaning history for personal analysis.
- **Enterprise Ready**: Multi-stage Podman build and automatic configuration validation.

## 🛠️ Bot Commands

- `/start` - Initial setup, shows your User ID.
- `/help` - Usage instructions and command list.
- `/stats` - View your personal cleaning statistics in-chat.

## 🚀 Quick Start

1. **Clone & Configure**:
   - Copy `.env.example` to `.env`.
   - Set `TELOXIDE_TOKEN`, `BOT_USERNAME`, and `ADMIN_ID`.
   - **Important**: Generate a random `COOKIE_KEY` for session persistence.
   - (Optional) Set `AI_API_KEY`, `AI_API_BASE`, and `AI_MODEL` for AI Deep Scan.

2. **Run Locally**:
   ```bash
   cargo run --release
   ```
   *For containerized deployment, refer to the architecture documentation.*

## 🏗️ Technical Architecture

- **Core**: Rust 2021, Teloxide 0.17 (Bot).
- **Database**: sqlx::Any (SQLite/PostgreSQL) with dynamic backend detection.
- **Stability**: Zero-panic core logic with comprehensive `tracing` instrumentation.

## 📝 License

MIT

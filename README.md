# ClearURLs Telegram Bot 🛡️

A professional, high-performance Rust-based Telegram bot that automatically removes tracking parameters from URLs using the ClearURLs ruleset. Featuring a real-time, accessible web dashboard and advanced management features.

## 🌟 Advanced Features

- **Real-Time Dashboard**: Updates stats and history instantly via **Server-Sent Events (SSE)** without page reloads.
- **Telegram Web App Integration**: Access the full management dashboard directly inside Telegram.
- **Modern UI/UX with Dark Mode**: Responsive design that automatically adapts to system themes (Light/Dark).
- **Multi-Language Support**: Full i18n support for Italian and English.
- **Granular Control**: Per-chat configuration (Reply/Delete modes) and custom tracking parameter removal.
- **Deep Auditing**: Track which provider (Amazon, Google, etc.) cleaned each link.
- **CSV Export**: Download your full cleaning history for personal analysis.
- **Enterprise Ready**: Multi-stage Docker build and automatic configuration validation.

## 🛠️ Bot Commands

- `/start` - Initial setup, shows your User ID and provides access to the Web App.
- `/help` - Usage instructions and command list.
- `/stats` - View your personal cleaning statistics in-chat.

## 🚀 Quick Start

1. **Clone & Configure**:
   - Copy `.env.example` to `.env`.
   - Set `TELOXIDE_TOKEN`, `BOT_USERNAME`, `DASHBOARD_URL`, and `ADMIN_ID`.
2. **Setup Telegram Domain**:
   - Use `/setdomain` in [@BotFather](https://t.me/BotFather) to point to your `DASHBOARD_URL`.
3. **Run with Docker**:
   ```bash
   docker-compose up --build
   ```
   *Or locally with `cargo run --release`.*

## 🏗️ Technical Architecture

- **Backend**: Rust 2021, Axum (Web), Teloxide (Bot).
- **Real-time**: Async broadcast channels with SSE streaming.
- **Frontend**: Accessible HTML5/JS with Chart.js and native Dark Mode support.
- **Database**: SQLite with SQLx and automatic schema migrations.
- **Reliability**: Config validation on startup and `/health` endpoint for monitoring.

## 📝 License

MIT

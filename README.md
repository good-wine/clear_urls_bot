# ClearURLs Telegram Bot 🛡️

A high-performance Rust-based Telegram bot that automatically removes tracking parameters from URLs using the ClearURLs ruleset. Features a secure web dashboard for user management and real-time statistics.

## Features

- **Automatic Sanitization**: Detects and cleans URLs in private chats, groups, and channels.
- **Media Support**: Automatically cleans links found in **captions** of photos, videos, documents, and audio files.
- **Auto-Updating Rules**: Background task refreshes the ClearURLs ruleset every 24 hours without service interruption.
- **Real-time Statistics**: Tracks the total number of links cleaned per user, visible both on the web dashboard and via bot commands.
- **Web Dashboard**: Secure Material-style interface to manage personal settings and view cleaning stats.
- **Action Modes**:
  - `Reply`: The bot replies with the cleaned version of the links.
  - `Delete & Repost`: Deletes the original message and posts a clean version (requires "Delete Messages" permission).
- **Global Toggle & Whitelist**: Enable/disable the bot or specify domains that should never be cleaned.

## Bot Commands

- `/start` - Initial setup and direct link to the secure dashboard.
- `/help` - Show usage instructions and available commands.
- `/stats` - View your personal cleaning statistics directly in Telegram.

## Setup

1. **Clone the repository**.
2. **Configure Environment**:
   - Copy `.env.example` to `.env`.
   - Create a bot via [@BotFather](https://t.me/BotFather) and get the token.
   - Set `TELOXIDE_TOKEN`, `BOT_USERNAME`, and `DASHBOARD_URL` in `.env`.
   - *Note: For local development, use `http://127.0.0.1:3000`.*
3. **Setup Telegram Login**:
   - In @BotFather, use `/setdomain` to link your `DASHBOARD_URL` (e.g., `http://127.0.0.1:3000`) to your bot. This is required for the Telegram Login widget.
4. **Run the application**:
   ```bash
   cargo run --release
   ```

## Technical Architecture

- **Language**: Rust (Edition 2021)
- **Telegram Framework**: [teloxide](https://github.com/teloxide/teloxide)
- **Web Server**: [axum](https://github.com/tokio-rs/axum) with `tower-http` security layers.
- **Security**:
  - Strict **Content Security Policy (CSP)** for Telegram OAuth.
  - Signed, HttpOnly, and SameSite session cookies.
  - HMAC-SHA256 authentication verification with replay attack protection.
- **Database**: [sqlx](https://github.com/launchbadge/sqlx) (SQLite) with automatic migrations.
- **Rules Engine**: Concurrent regex-based engine with `RwLock` for zero-downtime updates.

## License

MIT
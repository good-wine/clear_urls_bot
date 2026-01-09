# ClearURLs Telegram Bot

A Rust-based Telegram bot that automatically removes tracking parameters from URLs using the ClearURLs ruleset. Includes a web dashboard for configuration and management.

## Features

- **Automatic Sanitization**: Detects URLs in private chats, groups, and channels and strips tracking parameters.
- **ClearURLs Rules**: Uses the official `data.min.json` from the ClearURLs project.
- **Web Dashboard**: Manage your personal settings and see active chats via a Telegram Login interface.
- **Action Modes**:
  - `Reply`: The bot replies to the message with cleaned links.
  - `Delete & Repost`: The bot deletes the original message and posts the cleaned version (requires Delete Messages permission).
- **Global Toggle**: Enable or disable the bot for your own messages or specific chats.

## Setup

1. **Clone the repository** (or copy the files).
2. **Install Rust**: Ensure you have the latest stable Rust toolchain.
3. **Configure Environment**:
   - Copy `.env.example` to `.env`.
   - Create a bot via [@BotFather](https://t.me/BotFather) and get the token.
   - Set `TELOXIDE_TOKEN` and `BOT_USERNAME` in `.env`.
4. **Setup Telegram Login**:
   - In @BotFather, use `/setdomain` to link your dashboard domain (e.g., `https://your-domain.com` or `http://localhost:3000` for testing) to your bot.
5. **Run the application**:
   ```bash
   cargo run --release
   ```

## Configuration

The bot uses SQLite by default (`bot.db`). On first run, it will automatically create the necessary tables.

## Dashboard

Access the dashboard at `http://localhost:3000` (or your configured `SERVER_ADDR`). Log in with Telegram to:
- Toggle the bot globally for your account.
- Switch between "Reply" and "Delete & Repost" modes.
- View chats where the bot has been active.

## Technical Details

- **Language**: Rust
- **Telegram Library**: [teloxide](https://github.com/teloxide/teloxide)
- **Web Framework**: [axum](https://github.com/tokio-rs/axum)
- **Database**: [sqlx](https://github.com/launchbadge/sqlx) (SQLite)
- **Rules Engine**: Custom regex-based implementation parsing ClearURLs JSON.

## License

MIT

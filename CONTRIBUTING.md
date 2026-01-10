# Contributing to ClearURLs Bot 🛡️

Thank you for your interest in contributing! We welcome all contributions that help make this project more robust and feature-rich.

## 🛠️ Development Setup

1. **Prerequisites**: 
   - [Rust](https://www.rust-lang.org/tools/install) (latest stable)
   - [Docker](https://docs.docker.com/get-docker/) & [Docker Compose](https://docs.docker.com/compose/install/)
   - [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (for WASM changes)

2. **Clone & Config**:
   ```bash
   git clone https://github.com/yourusername/clear_urls_bot.git
   cd clear_urls_bot
   cp .env.example .env
   ```

3. **Running Locally**:
   - Start the database (SQLite by default): `cargo run`
   - Run the web dashboard: `cargo run --bin clear_urls_bot` (includes bot and web)

## 🧪 Testing & Quality

Always run these commands before submitting a PR:
- **Format**: `cargo fmt`
- **Lint**: `cargo clippy`
- **Check Targets**: `cargo check --all-targets --features vercel`
- **Test**: `cargo test`

## 📬 Pull Request Process

1. Create a branch: `git checkout -b feature/your-feature-name`.
2. Ensure documentation is updated for any new features or API changes.
3. Provide a clear description of the changes in the PR.
4. Link any related issues.

## ⚖️ Code of Conduct

Please be respectful and professional in all interactions within this repository.

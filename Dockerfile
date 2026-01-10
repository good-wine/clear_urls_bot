# --- Build Stage ---
FROM rust:1.84-slim-bookworm as builder

# Optimization: install dependencies separately to cache layers
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app
COPY . .

# Build the release binary
RUN cargo build --release --bin clear_urls_bot

# --- Runtime Stage ---
FROM debian:bookworm-slim

# Create a non-root user for security
RUN groupadd -r clearurls && useradd -r -g clearurls clearurls

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy assets and binary
COPY --from=builder /usr/src/app/target/release/clear_urls_bot /usr/local/bin/clear_urls_bot
COPY --from=builder /usr/src/app/templates ./templates

# Ensure the database file can be written if using SQLite
RUN touch bot.db && chown clearurls:clearurls bot.db && chown -R clearurls:clearurls /app

USER clearurls

# Healthcheck: ensure the web server is responding
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

EXPOSE 3000

ENV APP_ENV=production
ENV RUST_LOG=clear_urls_bot=info

CMD ["clear_urls_bot"]
# Build Stage
FROM rust:1.75-slim as builder

WORKDIR /usr/src/app
COPY . .

# Install dependencies for SQLx and other crates
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Build only the persistent bot binary for the container environment
RUN cargo build --release --bin clear_urls_bot

# Final Stage
FROM debian:bookworm-slim

WORKDIR /app
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy binary and templates
COPY --from=builder /usr/src/app/target/release/clear_urls_bot /usr/local/bin/clear_urls_bot
COPY --from=builder /usr/src/app/templates ./templates

EXPOSE 3000

# The bot binary will run both the bot loop and the web server if run locally
CMD ["clear_urls_bot"]

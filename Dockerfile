# Build Stage
FROM rust:1.75-slim as builder

WORKDIR /usr/src/app
COPY . .

# Install dependencies for SQLx and other crates
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

RUN cargo build --release

# Final Stage
FROM debian:bookworm-slim

WORKDIR /app
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/clear_urls_bot .
COPY --from=builder /usr/src/app/templates ./templates

EXPOSE 3000

CMD ["./clear_urls_bot"]

# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.toml
COPY src src

# Build application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install ca-certificates for HTTPS
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy binary from builder (built binary name is `qwesty`)
COPY --from=builder /app/target/release/qwesty /app/qwesty

# Create data directory
RUN mkdir -p /data

# No runtime ENV variables required - use config.toml instead

# Run the application
CMD ["/app/qwesty"]

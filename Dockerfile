# Build stage
FROM rust:1.83-slim as builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates

# Build release binary
RUN cargo build --release -p jst-server

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/jst-server /app/jst-server

ENV PORT=8080
EXPOSE 8080

USER nobody:nogroup
CMD ["/app/jst-server"]

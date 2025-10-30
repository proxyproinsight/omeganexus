# Multi-stage build for optimal size
FROM rust:1.81-slim as builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source
COPY src ./src
COPY migrations ./migrations

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/omega9-nexus /app/omega9-nexus

# Copy static assets
COPY static ./static

# Create non-root user
RUN useradd -m -u 1000 nexususer && \
    chown -R nexususer:nexususer /app

USER nexususer

EXPOSE 8080

CMD ["/app/omega9-nexus"]

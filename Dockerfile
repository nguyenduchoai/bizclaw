# ═══════════════════════════════════════════════════════════════
# BizClaw AI Agent Platform — Multi-stage Docker Build
# Self-hosted on Pi, VPS, or any Linux machine
# ═══════════════════════════════════════════════════════════════

# Stage 1: Build
FROM rust:latest AS builder

WORKDIR /build

# Copy workspace Cargo files first (for dependency caching)
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY src/ src/
COPY data/ data/

# Build release binaries
RUN cargo build --release --bin bizclaw --bin bizclaw-platform

# Stage 2: Runtime — use trixie to match glibc from rust:latest (2.40)
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries
COPY --from=builder /build/target/release/bizclaw /usr/local/bin/bizclaw
COPY --from=builder /build/target/release/bizclaw-platform /usr/local/bin/bizclaw-platform

# Create data directory
RUN mkdir -p /root/.bizclaw

# Environment — GMT+7
ENV BIZCLAW_CONFIG=/root/.bizclaw/config.toml
ENV RUST_LOG=info
ENV TZ=Asia/Ho_Chi_Minh

# Expose ports: platform admin (3001) + tenant gateways (10001-10010)
EXPOSE 3001 10001 10002 10003 10004 10005

# Health check
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:3001/health || exit 1

# Default: run the platform
ENTRYPOINT ["bizclaw-platform"]
CMD ["--port", "3001", "--bizclaw-bin", "/usr/local/bin/bizclaw"]

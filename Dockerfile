# Rustor Docker Image
#
# Build:
#   docker build -t rustor .
#
# Usage:
#   docker run -v $(pwd):/src rustor /src --dry-run
#   docker run -v $(pwd):/src rustor /src --fix

# Build stage
FROM rust:1.75-slim as builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy source
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build release binary
RUN cargo build --release --bin rustor

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies (minimal)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/rustor /usr/local/bin/rustor

# Set working directory
WORKDIR /src

# Default command
ENTRYPOINT ["rustor"]
CMD ["--help"]

# Labels
LABEL org.opencontainers.image.title="Rustor"
LABEL org.opencontainers.image.description="PHP refactoring tool"
LABEL org.opencontainers.image.source="https://github.com/rustor/rustor"
LABEL org.opencontainers.image.licenses="MIT"

# Build stage
FROM rust:1.75-bookworm as builder

# Install protobuf compiler
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY lib ./lib
COPY arenabuddy/arenabuddy_core ./arenabuddy/arenabuddy_core
COPY arenabuddy/arenabuddy_data ./arenabuddy/arenabuddy_data
COPY arenabuddy/arenabuddy_backend ./arenabuddy/arenabuddy_backend

# Build the backend service
WORKDIR /app/arenabuddy/arenabuddy_backend
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1001 -s /bin/bash arenabuddy

# Copy the binary from builder
COPY --from=builder /app/target/release/arenabuddy_backend /usr/local/bin/arenabuddy_backend

# Copy migrations (if needed for embedded mode)
COPY --from=builder /app/arenabuddy/arenabuddy_data/migrations /app/migrations

# Set ownership
RUN chown -R arenabuddy:arenabuddy /app

# Switch to non-root user
USER arenabuddy

# Set working directory
WORKDIR /app

# Environment variables (can be overridden)
ENV RUST_LOG=arenabuddy_backend=info,arenabuddy_data=info,arenabuddy_core=info
ENV GRPC_SERVER_ADDR=0.0.0.0:50051

# Expose gRPC port
EXPOSE 50051

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD nc -z localhost 50051 || exit 1

# Run the service
ENTRYPOINT ["/usr/local/bin/arenabuddy_backend"]
# Stage 1: Build Frontend
FROM node:20-slim AS frontend-builder
WORKDIR /app
COPY package.json package-lock.json pnpm-lock.yaml ./
RUN npm install -g pnpm && pnpm install --frozen-lockfile
COPY . .
RUN pnpm run build

# Stage 2: Build Rust Backend
FROM rust:1.75-slim AS backend-builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy Cargo files first for caching
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release --no-default-features --features web-server 2>/dev/null || true

# Copy source and build
COPY src-tauri/ .
RUN cargo build --release --bin antigravity-server --no-default-features --features web-server

# Stage 3: Production
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# Copy Rust backend binary
COPY --from=backend-builder /app/target/release/antigravity-server /usr/local/bin/

# Copy frontend static files
COPY --from=frontend-builder /app/dist ./dist

# Create data directory
RUN mkdir -p /data

# Environment
ENV PORT=3000
ENV DATA_DIR=/data
ENV STATIC_PATH=/app/dist

EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/healthz || exit 1

CMD ["antigravity-server", "--port", "3000", "--data-dir", "/data"]

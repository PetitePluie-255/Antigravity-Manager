# Stage 1: Build Frontend
FROM node:20-slim AS frontend-builder
WORKDIR /app

# Install pnpm
RUN npm install -g pnpm

# Copy package files
COPY package.json pnpm-lock.yaml ./

# Install dependencies
RUN pnpm install --frozen-lockfile

# Copy source
COPY . .

# Build frontend
RUN pnpm run build

# Stage 2: Build Rust Backend
FROM rustlang/rust:nightly-slim AS backend-builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy Cargo files first for caching
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    mkdir -p src/bin && echo "fn main() {}" > src/bin/server.rs && \
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

# Environment variables
ENV PORT=3000
ENV DATA_DIR=/data
ENV STATIC_DIR=/app/dist
ENV BIND_ADDRESS=0.0.0.0

EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/healthz || exit 1

# Run server
CMD ["antigravity-server", "--port", "3000", "--data-dir", "/data"]

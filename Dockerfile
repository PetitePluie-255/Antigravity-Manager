# syntax=docker/dockerfile:1.4

# Stage 1: Build Frontend
FROM node:20-slim AS frontend-builder
WORKDIR /app

# Install pnpm
RUN npm install -g pnpm

# Copy package files first (for dependency caching)
COPY package.json pnpm-lock.yaml ./

# Install dependencies (cached if package files unchanged)
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile

# Copy source files
COPY src/ ./src/
COPY public/ ./public/
COPY index.html vite.config.ts tsconfig*.json tailwind.config.js postcss.config.js ./

# Build frontend
RUN pnpm run build

# Stage 2: Build Rust Backend
FROM rustlang/rust:nightly-slim AS backend-builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy Cargo files first for dependency caching
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock ./

# Create dummy source files for dependency caching
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/server.rs && \
    echo "pub fn lib() {}" > src/lib.rs

# Build dependencies only (cached if Cargo.toml/lock unchanged)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --no-default-features --features web-server 2>/dev/null || true

# Copy actual source code
COPY src-tauri/ .

# Build the actual binary (uses cached dependencies)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin antigravity-server --no-default-features --features web-server && \
    cp target/release/antigravity-server /antigravity-server

# Stage 3: Production
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# Copy Rust backend binary
COPY --from=backend-builder /antigravity-server /usr/local/bin/

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

# syntax=docker/dockerfile:1.4

# ============================================
# Stage 1: Build Frontend (React + Vite)
# ============================================
FROM node:20-slim AS frontend-builder
WORKDIR /app

# Install pnpm
RUN npm install -g pnpm

# Copy package files first (for dependency caching)
COPY web/package.json web/pnpm-lock.yaml ./

# Install dependencies (cached if package files unchanged)
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile

# Copy source files
COPY web/src/ ./src/
COPY web/public/ ./public/
COPY web/index.html web/vite.config.ts web/tsconfig*.json web/tailwind.config.js web/postcss.config.cjs ./

# Build frontend for production
RUN pnpm run build

# ============================================
# Stage 2: Build Rust Backend (Axum Server)
# ============================================
FROM rustlang/rust:nightly-slim AS backend-builder
WORKDIR /app

# Install build dependencies (cached)
RUN --mount=type=cache,target=/var/cache/apt \
    --mount=type=cache,target=/var/lib/apt \
    apt-get update && \
    apt-get install -y pkg-config libssl-dev

# Copy Cargo files first for dependency caching
COPY server/Cargo.toml server/Cargo.lock ./

# Create dummy source files for dependency caching
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn lib() {}" > src/lib.rs

# Build dependencies only (cached if Cargo.toml/lock unchanged)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release 2>/dev/null || true

# Remove dummy files and clean target to force rebuild
RUN rm -rf src target

# Copy actual source code
COPY server/src ./src

# Build the actual binary
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release && \
    cp target/release/antigravity-server /antigravity-server && \
    strip /antigravity-server

# ============================================
# Stage 3: Production Image
# ============================================
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
    CMD curl -f http://localhost:3000/api/proxy/status || exit 1

# Run server
CMD ["antigravity-server", "--port", "3000", "--data-dir", "/data"]

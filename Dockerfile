# Stage 1: Build Frontend
FROM node:20-slim AS frontend-builder
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build

# Stage 2: Build Backend
FROM node:20-slim AS backend-builder
WORKDIR /app/server
COPY server/package.json server/package-lock.json ./
# Install build tools for native modules (better-sqlite3)
RUN apt-get update && \
    apt-get install -y python3 make g++ build-essential && \
    rm -rf /var/lib/apt/lists/*
RUN npm ci
COPY server/ .
RUN npm run build
# Prune dev dependencies (keep native modules)
RUN npm prune --production
# Prune dev dependencies so we can copy only prod deps later

# Stage 3: Production
FROM node:20-slim
WORKDIR /app

# Install required build tools for better-sqlite3 and healthcheck
# Install required build tools for better-sqlite3 and healthcheck
RUN apt-get update && \
    apt-get install -y python3 make g++ build-essential curl python-is-python3 && \
    rm -rf /var/lib/apt/lists/*


# Copy backend artifacts
COPY --from=backend-builder /app/server/dist ./server
COPY --from=backend-builder /app/server/package.json ./server/
COPY --from=backend-builder /app/server/node_modules ./server/node_modules

WORKDIR /app/server
# No need to install or rebuild, just use the copied modules

# Copy frontend
WORKDIR /app
COPY --from=frontend-builder /app/dist ./dist

# Create data directory
RUN mkdir -p /data

# Environment
ENV NODE_ENV=production
ENV PORT=3000
ENV DATA_DIR=/data
ENV STATIC_PATH=/app/dist

EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/ || exit 1

CMD ["node", "server/index.js"]

# Stage 1: Build Frontend
FROM node:20-slim AS frontend-builder
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
COPY package.json pnpm-lock.yaml ./
RUN npm install -g pnpm
RUN pnpm install --frozen-lockfile
COPY . .
RUN pnpm run build

# Stage 2: Build Backend
FROM node:20-slim AS backend-builder
WORKDIR /server
COPY server/package.json server/pnpm-lock.yaml ./
COPY server/package.json server/pnpm-lock.yaml ./
RUN npm install -g pnpm
RUN pnpm install --frozen-lockfile
COPY server/ .
RUN pnpm run build
# Prune dev dependencies so we can copy only prod deps later
RUN pnpm prune --prod --no-optional

# Stage 3: Production
FROM node:20-slim
WORKDIR /app

# Install required build tools for better-sqlite3 and healthcheck
RUN apt-get update && \
    apt-get install -y python3 make g++ build-essential curl && \
    rm -rf /var/lib/apt/lists/*

# Copy backend
COPY --from=backend-builder /server/dist ./server
COPY --from=backend-builder /server/package.json ./server/
# Copy pre-built node_modules with native bindings
COPY --from=backend-builder /server/node_modules ./server/node_modules
WORKDIR /app/server

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

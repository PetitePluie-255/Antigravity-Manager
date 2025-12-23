# Stage 1: Build Frontend
FROM node:20-alpine AS frontend-builder
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
RUN npm install -g pnpm
RUN pnpm install --frozen-lockfile
COPY . .
RUN pnpm run build

# Stage 2: Build Backend
FROM node:20-alpine AS backend-builder
WORKDIR /server
COPY server/package.json server/pnpm-lock.yaml ./
RUN npm install -g pnpm
RUN pnpm install --frozen-lockfile
COPY server/ .
RUN pnpm run build

# Stage 3: Production
FROM node:20-alpine
WORKDIR /app

# Install required build tools for better-sqlite3
RUN apk add --no-cache python3 make g++

# Copy backend
COPY --from=backend-builder /server/dist ./server
COPY --from=backend-builder /server/package.json /server/pnpm-lock.yaml ./server/
WORKDIR /app/server
RUN npm install -g pnpm
RUN pnpm install --frozen-lockfile --prod

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
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/ || exit 1

CMD ["node", "server/index.js"]

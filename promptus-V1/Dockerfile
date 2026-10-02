# syntax=docker/dockerfile:1.7

# ----------------------------------------------------------------------------
# Promptus — production image
# ----------------------------------------------------------------------------
# Multi-stage build:
#   1. deps:    install pnpm + production+dev deps for the build
#   2. builder: build the Next.js app using "standalone" output
#   3. realtime: WebSocket server (target `realtime`)
#   4. runner:  small runtime image, runs `node server.js` (default target)
# ----------------------------------------------------------------------------

FROM node:22-alpine AS deps
RUN corepack enable && corepack prepare pnpm@10.33.0 --activate
WORKDIR /app
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile

FROM node:22-alpine AS builder
RUN corepack enable && corepack prepare pnpm@10.33.0 --activate
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .
ENV NEXT_TELEMETRY_DISABLED=1
ENV NODE_ENV=production
RUN pnpm build

# Serveur temps réel (WebSocket) : script TypeScript autonome, exécuté par Node.
FROM node:22-alpine AS realtime
WORKDIR /app
ENV NODE_ENV=production
COPY --from=deps /app/node_modules ./node_modules
COPY package.json ./
COPY realtime ./realtime
USER node
EXPOSE 3001
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD wget -qO- http://localhost:3001/health >/dev/null || exit 1
CMD ["node", "--experimental-strip-types", "--no-warnings", "realtime/server.mts"]

FROM node:22-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1
ENV PORT=3000
ENV HOSTNAME=0.0.0.0
RUN addgroup --system --gid 1001 nodejs && \
    adduser --system --uid 1001 nextjs

# Static assets
COPY --from=builder /app/public ./public

# Standalone server bundle
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static

# Drizzle migrations are needed at startup to apply pending changes.
COPY --from=builder --chown=nextjs:nodejs /app/src/lib/db/migrations ./src/lib/db/migrations
COPY --from=builder --chown=nextjs:nodejs /app/drizzle.config.ts ./drizzle.config.ts

USER nextjs
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD wget -qO- http://localhost:3000/api/audio/assets >/dev/null || exit 1
CMD ["node", "server.js"]

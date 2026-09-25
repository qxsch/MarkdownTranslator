# syntax=docker/dockerfile:1

FROM node:24-slim AS build
WORKDIR /app
# Optional package proxy/mirror (npm registry URL ending with /). Default: public npm registry.
ARG NPM_REGISTRY=https://registry.npmjs.org/
COPY package.json package-lock.json ./
# Lockfile tarball URLs point at registry.npmjs.org; resolve them against NPM_REGISTRY instead.
RUN sed -i -E "s#https://registry\.npmjs\.org/#${NPM_REGISTRY}#g" package-lock.json \
 && npm config set registry "$NPM_REGISTRY" && npm ci --no-audit --no-fund
COPY tsconfig.json tsconfig.build.json ./
COPY src ./src
RUN npx tsc -p tsconfig.build.json && npm prune --omit=dev

FROM node:24-slim
ENV NODE_ENV=production PORT=8080
WORKDIR /app
COPY --from=build --chown=node:node /app/node_modules ./node_modules
COPY --from=build --chown=node:node /app/dist ./dist
COPY --chown=node:node package.json ./
COPY --chown=node:node config ./config
USER node
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s CMD ["node", "-e", "fetch('http://127.0.0.1:'+(process.env.PORT||8080)+'/healthz').then(r=>process.exit(r.ok?0:1),()=>process.exit(1))"]
CMD ["node", "dist/src/main.js"]

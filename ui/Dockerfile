# syntax=docker/dockerfile:1.7

FROM node:22-bookworm-slim AS builder

WORKDIR /app

COPY ui/package.json ui/package-lock.json ./
RUN npm ci

COPY ui/ ./
RUN npm run build

FROM nginxinc/nginx-unprivileged:stable

COPY ui/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/dist /usr/share/nginx/html

LABEL org.opencontainers.image.title="Autofile UI" \
      org.opencontainers.image.description="Autofile web UI" \
      org.opencontainers.image.source="https://github.com/joekhoobyar/autofile" \
      org.opencontainers.image.url="https://autofile.dev/" \
      org.opencontainers.image.documentation="https://autofile.dev/" \
      org.opencontainers.image.vendor="Joe Khoobyar" \
      org.opencontainers.image.licenses="AGPL-3.0-only"

EXPOSE 8080

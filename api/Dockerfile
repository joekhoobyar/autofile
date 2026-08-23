# syntax=docker/dockerfile:1.7

ARG RUST_BASE_IMAGE=ghcr.io/joekhoobyar/autofile-api-rust-base:latest
ARG API_RUNTIME_BASE_IMAGE=ghcr.io/joekhoobyar/autofile-api-runtime-base:latest

FROM ${RUST_BASE_IMAGE} AS planner

WORKDIR /app
COPY api/Cargo.toml api/Cargo.lock ./
RUN mkdir -p src && printf 'fn main() {}\n' > src/main.rs
RUN cargo chef prepare --recipe-path recipe.json

FROM ${RUST_BASE_IMAGE} AS builder

WORKDIR /app
COPY --from=planner /app/recipe.json ./recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

COPY api/Cargo.toml api/Cargo.lock ./
COPY LICENSE ../
COPY api/diesel.toml ./
COPY api/src ./src
COPY api/migrations ./migrations

RUN cargo build --release \
    && cp /app/target/release/autofile-api /app/autofile-api

FROM ${API_RUNTIME_BASE_IMAGE} AS runtime

RUN useradd -m -u 1000 -s /bin/bash appuser

WORKDIR /app
COPY --from=builder /app/autofile-api /usr/local/bin/autofile-api

USER 1000
EXPOSE 8000

LABEL org.opencontainers.image.title="Autofile API" \
      org.opencontainers.image.description="Autofile backend API service" \
      org.opencontainers.image.source="https://github.com/joekhoobyar/autofile" \
      org.opencontainers.image.url="https://github.com/joekhoobyar/autofile" \
      org.opencontainers.image.documentation="https://github.com/joekhoobyar/autofile" \
      org.opencontainers.image.vendor="Joe Khoobyar" \
      org.opencontainers.image.licenses="AGPL-3.0-only"

ENTRYPOINT ["tini", "--"]
CMD ["/usr/local/bin/autofile-api"]

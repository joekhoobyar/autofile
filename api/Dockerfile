# syntax=docker/dockerfile:1.7

ARG BUILDPLATFORM
ARG TARGETPLATFORM

FROM debian:trixie-slim AS rust-base

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --profile minimal --default-toolchain stable

RUN cargo install cargo-chef

FROM rust-base AS planner

WORKDIR /app
COPY api/Cargo.toml api/Cargo.lock ./
RUN mkdir -p src && printf 'fn main() {}\n' > src/main.rs
RUN cargo chef prepare --recipe-path recipe.json

FROM rust-base AS builder

ARG TARGETPLATFORM

WORKDIR /app
COPY --from=planner /app/recipe.json ./recipe.json

RUN --mount=type=cache,id=autofile-cargo-registry-${TARGETPLATFORM},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=autofile-cargo-git-${TARGETPLATFORM},target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=autofile-target-${TARGETPLATFORM},target=/app/target,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json

COPY api/Cargo.toml api/Cargo.lock ./
COPY api/diesel.toml ./
COPY api/src ./src
COPY api/migrations ./migrations

RUN --mount=type=cache,id=autofile-cargo-registry-${TARGETPLATFORM},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=autofile-cargo-git-${TARGETPLATFORM},target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=autofile-target-${TARGETPLATFORM},target=/app/target,sharing=locked \
    cargo build --release \
    && cp /app/target/release/autofile-api /app/autofile-api

FROM debian:trixie-slim AS runtime

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    libpq5 \
    imagemagick \
    libreoffice \
    pandoc \
    poppler-utils \
    weasyprint \
    texlive-latex-base \
    texlive-xetex \
    fonts-dejavu \
    fonts-liberation \
    tini \
    && rm -rf /var/lib/apt/lists/*

RUN command -v magick \
 && command -v soffice \
 && command -v pandoc \
 && command -v pdftocairo \
 && command -v pdftotext \
 && command -v pdfinfo \
 && command -v weasyprint \
 && command -v pdflatex \
 && command -v xelatex

RUN useradd -m -u 1000 -s /bin/bash appuser

WORKDIR /app
COPY --from=builder /app/autofile-api /usr/local/bin/autofile-api

USER appuser
EXPOSE 8000

ENTRYPOINT ["tini", "--"]
CMD ["/usr/local/bin/autofile-api"]

# syntax=docker/dockerfile:1.7

FROM debian:trixie-slim AS builder

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

WORKDIR /app

COPY api/Cargo.toml api/Cargo.lock ./
COPY api/diesel.toml ./
COPY api/src ./src
COPY api/migrations ./migrations

RUN cargo build --release

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
COPY --from=builder /app/target/release/autofile-api /usr/local/bin/autofile-api

USER appuser
EXPOSE 8000

ENTRYPOINT ["tini", "--"]
CMD ["/usr/local/bin/autofile-api"]

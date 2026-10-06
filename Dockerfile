# Build stage
FROM rust:1-slim-bookworm AS build
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --locked && \
    cp target/release/tickit /build/tickit

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates libssl3 tzdata wget \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 tickit \
    && useradd --uid 10001 --gid 10001 --create-home tickit \
    && mkdir -p /app/assets /var/lib/tickit \
    && chown -R tickit:tickit /app /var/lib/tickit
COPY --from=build --chown=tickit:tickit /build/tickit /usr/local/bin/tickit
COPY --chown=tickit:tickit assets /app/assets
WORKDIR /app
USER tickit
EXPOSE 8081
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://127.0.0.1:8081/healthz > /dev/null || exit 1
CMD ["tickit"]
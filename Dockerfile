# 1. Build the web app (bun) → apps/web/dist
FROM oven/bun:1 AS web
WORKDIR /app
COPY apps/web ./apps/web
RUN cd apps/web && bun install --frozen-lockfile && bun run build

# 2. Build the server (release), with the web app baked into the binary
FROM rust:1.93-bookworm AS server
ENV PKG_CONFIG_ALLOW_CROSS=1
WORKDIR /usr/src/koji
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY apps/koji-cli ./apps/koji-cli
COPY apps/koji-server ./apps/koji-server
# Stage the prebuilt web app into the folder rust-embed embeds from.
COPY --from=web /app/apps/web/dist ./crates/koji-service/web
RUN cargo install --path apps/koji-server --locked

# 3. Runtime — just the single binary (the web app lives inside it)
FROM debian:bookworm-slim AS runner
COPY --from=server /usr/local/cargo/bin/koji /usr/local/bin/koji
# External plugins: mount a directory at /plugins (see docker-compose.example.yml)
# and the koji-plugins registry scans it at startup. No plugins ship in-image —
# routing/clustering are native Rust (tsp-mt, crucible).
ENV KOJI_PLUGINS_DIR=/plugins
RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*
CMD ["koji"]

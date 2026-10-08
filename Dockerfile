# 1. Build the web app (bun) → apps/web/dist
FROM oven/bun:1 AS web
WORKDIR /app
COPY apps/web ./apps/web
RUN cd apps/web && bun install --frozen-lockfile && bun run build

# 2. Build the server (release), with the web app baked into the binary.
# cargo-chef builds the dependencies from a recipe of the manifests alone, so
# that layer stays cached until Cargo.toml or Cargo.lock change.
FROM lukemathwalker/cargo-chef:0.1.78-rust-1.99-bookworm AS chef
WORKDIR /usr/src/koji

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY apps/koji-cli ./apps/koji-cli
COPY apps/koji-server ./apps/koji-server
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS server
ENV PKG_CONFIG_ALLOW_CROSS=1
COPY --from=planner /usr/src/koji/recipe.json recipe.json
RUN cargo chef cook --release --locked -p koji --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY apps/koji-cli ./apps/koji-cli
COPY apps/koji-server ./apps/koji-server
# Stage the prebuilt web app into the folder rust-embed embeds from.
COPY --from=web /app/apps/web/dist ./crates/koji-service/web
RUN cargo build --release --locked -p koji

# 3. Runtime — just the single binary (the web app lives inside it)
FROM debian:bookworm-slim AS runner
COPY --from=server /usr/src/koji/target/release/koji /usr/local/bin/koji
# External plugins: mount a directory at /plugins (see docker-compose.example.yml)
# and the koji-plugins registry scans it at startup. No plugins ship in-image —
# routing/clustering are native Rust (tsp-mt, crucible).
ENV KOJI_PLUGINS_DIR=/plugins
RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*
CMD ["koji"]

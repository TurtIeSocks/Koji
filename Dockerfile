FROM node:22-alpine AS client
WORKDIR /app
COPY ./apps/web-client .
RUN yarn install
RUN yarn build

FROM rust:1.93-bookworm AS server
ENV PKG_CONFIG_ALLOW_CROSS=1
WORKDIR /usr/src/koji
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY apps/koji-cli ./apps/koji-cli
COPY apps/koji-server ./apps/koji-server
RUN apt-get update && apt-get install -y
RUN cargo install --path apps/koji-server --locked

FROM debian:bookworm AS or-tools
RUN mkdir -p /algorithms/src/routing/plugins
RUN apt-get update && apt-get install -y
RUN apt install -y curl build-essential cmake lsb-release
RUN ldconfig
COPY ./or-tools/src ./src
RUN curl -L https://github.com/google/or-tools/releases/download/v9.5/or-tools_amd64_debian-11_cpp_v9.5.2237.tar.gz -o ortools.tar.gz
RUN cat ortools.tar.gz | tar -xzf - && \
    mv or-tools_* or-tools && \
    cd or-tools && \
    mv /src/tsp/ ./examples/koji_tsp && \
    make build SOURCE=examples/koji_tsp/koji_tsp.cc && \
    mv ./examples/koji_tsp/build/bin/koji_tsp /algorithms/src/routing/plugins/tsp

FROM debian:bookworm-slim AS runner
COPY --from=or-tools /algorithms ./algorithms
COPY --from=or-tools /or-tools ./or-tools
COPY --from=client /app/dist ./dist
COPY --from=server /usr/local/cargo/bin/koji /usr/local/bin/koji
# Plugin registry: the `tsp` manifest points (absolutely) at the OR-Tools binary
# placed under /algorithms above. `KOJI_PLUGINS_DIR` tells the koji-plugins
# registry where to scan (post-P7 the in-tree filesystem scan was replaced by
# manifest discovery). Without this, the `tsp` routing optimizer is not found
# and routing silently degrades to the s2 fallback.
COPY plugins ./plugins
ENV KOJI_PLUGINS_DIR=/plugins
RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

CMD koji

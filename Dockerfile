FROM node:22-alpine as client
WORKDIR /app
COPY ./client .
RUN yarn install
RUN yarn build

FROM rust:1.71 as server
ENV PKG_CONFIG_ALLOW_CROSS=1
WORKDIR /usr/src/koji
COPY ./server .
RUN apt-get update && apt-get install -y
RUN cargo install --path . --locked

FROM debian:bullseye-slim as or-tools
RUN mkdir -p /algorithms/src/routing/plugins
RUN apt-get update && apt-get install -y
RUN apt install -y curl build-essential cmake lsb-release
RUN ldconfig
COPY ./or-tools/src ./src
RUN curl -L https://github.com/google/or-tools/releases/download/v9.10/or-tools_amd64_debian-11_cpp_v9.10.4067.tar.gz -o ortools.tar.gz
RUN cat ortools.tar.gz | tar -xzf - && \
    mv or-tools_* or-tools && \
    cd or-tools && \ 
    mv /src/tsp/ ./examples/koji_tsp && \
    make build SOURCE=examples/koji_tsp/koji_tsp.cc && \
    mv ./examples/koji_tsp/build/bin/koji_tsp /algorithms/src/routing/plugins/tsp

FROM debian:bullseye-slim as runner
COPY --from=or-tools /algorithms .
COPY --from=or-tools /or-tools .
COPY --from=client /app/dist ./dist
COPY --from=server /usr/local/cargo/bin/koji /usr/local/bin/koji

CMD koji

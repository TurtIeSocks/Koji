FROM node:18-alpine as client
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

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y
RUN apt install -y build-essential cmake lsb-release
RUN ldconfig
COPY --from=client /app/dist ./dist
COPY --from=server /usr/local/cargo/bin/koji /usr/local/bin/koji
RUN apt install curl -y
RUN apt install -y build-essential cmake lsb-release
RUN mkdir -p /algorithms/src/routing/plugins
COPY ./or-tools .
RUN ./or-tools/install.sh
CMD koji

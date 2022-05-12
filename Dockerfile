FROM node:16-alpine as client

WORKDIR /app
COPY package.json .
COPY yarn.lock .
RUN yarn install
COPY . .
RUN yarn build

FROM rust:1.60 as server
ENV PKG_CONFIG_ALLOW_CROSS=1
WORKDIR /usr/src/rdm-tools-2
COPY ./server .

RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y
RUN apt install default-libmysqlclient-dev -y
COPY --from=client /app/dist ./dist
COPY --from=server /usr/local/cargo/bin/rdm-tools-2 /usr/local/bin/rdm-tools-2

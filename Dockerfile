FROM node:16-alpine as client

WORKDIR /app
COPY package.json .
COPY yarn.lock .
RUN yarn install
COPY . .
RUN yarn build

FROM rust:1.60 as server
ENV PKG_CONFIG_ALLOW_CROSS=1
WORKDIR /usr/src/koji
COPY ./server .
RUN apt-get update && apt-get install -y
RUN apt-get install libcgal-dev -y

RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y
RUN apt install default-libmysqlclient-dev -y
COPY --from=client /app/dist ./dist
COPY --from=server /usr/local/cargo/bin/koji /usr/local/bin/koji

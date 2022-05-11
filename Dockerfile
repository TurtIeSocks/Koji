FROM rust:1.60 as build
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /usr/src/rdm-tools-2
COPY ./server .

RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y
RUN apt install default-libmysqlclient-dev -y
RUN apt install nodejs -y
RUN apt install npm -y
RUN apt install curl -y
RUN npm install -g yarn n
RUN n lts
COPY . .
RUN yarn install
COPY --from=build /usr/local/cargo/bin/rdm-tools-2 /usr/local/bin/rdm-tools-2

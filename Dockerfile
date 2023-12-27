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
RUN mkdir -p /algorithms/src/routing
COPY ./or-tools .
RUN curl -L https://github.com/google/or-tools/releases/download/v9.5/or-tools_amd64_debian-11_cpp_v9.5.2237.tar.gz -o ortools.tar.gz
RUN cat ortools.tar.gz | tar -xzf - && \
    cd or-tools_* && \ 
    mkdir examples/koji && \
    cp /tsp/tsp.cc ./examples/koji/koji.cc && \
    cp /tsp/CMakeLists.txt ./examples/koji/CMakeLists.txt && \
    make build SOURCE=examples/koji/koji.cc && \
    mv ./examples/koji/build/bin/koji /algorithms/src/routing/plugins/tsp
CMD koji

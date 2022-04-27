FROM node:16

ENV NPM_CONFIG_PREFIX=/home/node/.npm-global
ENV PATH=$PATH:/home/node/.npm-global/bin

WORKDIR /app

RUN npm install -g yarn

COPY package.json .
COPY yarn.lock .

RUN yarn install

COPY . .

RUN yarn build

# Kōji

## Installation

- Install Rust (basically just `apt update && apt upgrade` and `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- `cd server && cp .env.example .env` and fill out the .env
- temporary: set `NODE_ENV` to `development`
- `cd ../client && yarn install && yarn build`
- `cd ../server && cargo run -r` (you might have to `apt install pkg-config`)

# Kōji — one-command build.
#
# `make` (or `make build`) produces ONE self-contained binary at
# target/release/koji with the web app compiled into it (rust-embed). No more
# cd-ing between directories. Run `make help` for all targets.
#
# Requires: bun (frontend) + the Rust toolchain (server).

WEB_DIR   := apps/web
EMBED_DIR := crates/koji-service/web
BIN       := target/release/koji

.DEFAULT_GOAL := help
.PHONY: help web build release run dev dev-web dev-server install test clean

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-11s\033[0m %s\n", $$1, $$2}'

web: ## Build the frontend and stage it into the server's embed folder
	cd $(WEB_DIR) && bun install && bun run build
	find $(EMBED_DIR) -mindepth 1 ! -name .gitkeep -delete
	cp -R $(WEB_DIR)/dist/. $(EMBED_DIR)/

build: web ## Build the release binary with the web app embedded (deploy artifact)
	cargo build --release -p koji
	@echo "==> built $(BIN) (web embedded)"

release: build ## Alias for `build`

run: build ## Build, then run the release binary
	$(BIN)

dev: ## Run the frontend (Vite HMR) + server together for development
	$(MAKE) -j2 dev-web dev-server

dev-web:
	cd $(WEB_DIR) && bun run dev

dev-server:
	cargo run -p koji

install: web ## Install the koji binary via cargo (web embedded)
	cargo install --path apps/koji-server --locked

test: ## Run the Rust test suite
	cargo test

clean: ## Remove build artifacts (cargo + web dist + staged embed)
	cargo clean
	rm -rf $(WEB_DIR)/dist
	find $(EMBED_DIR) -mindepth 1 ! -name .gitkeep -delete

# Common development tasks. Run `make help` for the list.
.DEFAULT_GOAL := help
COMPOSE := docker compose -f infra/docker-compose.yml

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-16s %s\n", $$1, $$2}'

deps-up: ## Start MySQL, Redis, MinIO, Mailpit
	$(COMPOSE) up -d --wait

deps-down: ## Stop local dependencies (data is kept)
	$(COMPOSE) down

migrate: ## Apply database migrations
	cd backend && cargo run -q -p pos-api -- migrate

api: ## Run the API (http://127.0.0.1:8080)
	cd backend && cargo run -p pos-api -- serve

worker: ## Run the background worker
	cd backend && cargo run -p pos-worker

web: ## Run the frontend dev server (http://localhost:5173)
	cd frontend && npm run dev

guards: ## Regenerate tenant guard triggers in unapplied migrations
	cd backend && python3 scripts/gen_guards.py migrations/*.sql

check: ## Everything CI runs (needs MySQL; see docs/DEVELOPMENT.md)
	cd backend && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings \
		&& python3 scripts/gen_guards.py --check migrations/*.sql && python3 scripts/check_tenant_sql.py \
		&& cargo test --workspace
	cd frontend && npm run lint && npm run typecheck && npm test && npm run build

.PHONY: help deps-up deps-down migrate api worker web guards check

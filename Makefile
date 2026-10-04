.PHONY: dev dev-fe build build-fe test fmt clippy check clean \
        up down down-volumes logs ps docker-build

BACKEND_DIR  := backend
FRONTEND_DIR := frontend

# ── Development ──────────────────────────────────────────────────────────────

## dev: start the backend with --dev-mode (spawns a throwaway Postgres via Docker)
dev:
	cd $(BACKEND_DIR) && cargo run --bin testmodeller -- --dev-mode

## dev-release: dev mode, release build
dev-release:
	cd $(BACKEND_DIR) && cargo run --bin testmodeller --release -- --dev-mode

## dev-fe: start the Angular dev server (proxy to backend on :8080)
dev-fe:
	cd $(FRONTEND_DIR) && npx ng serve --proxy-config proxy.conf.json

# ── Build ─────────────────────────────────────────────────────────────────────

## build: debug build of all workspace members
build:
	cd $(BACKEND_DIR) && cargo build --workspace

## build-fe: production build of the Angular app
build-fe:
	cd $(FRONTEND_DIR) && npx ng build --configuration production

## build-release: optimised release build
build-release:
	cd $(BACKEND_DIR) && cargo build --workspace --release

# ── Quality ───────────────────────────────────────────────────────────────────

## fmt: format all backend Rust code in-place
fmt:
	cd $(BACKEND_DIR) && cargo fmt --all

## fmt-check: verify formatting without modifying files
fmt-check:
	cd $(BACKEND_DIR) && cargo fmt --all --check

## clippy: lint all targets, turning warnings into errors
clippy:
	cd $(BACKEND_DIR) && cargo clippy --all-targets -- -D warnings

## check: fmt-check + clippy (CI gate)
check: fmt-check clippy

# ── Tests ──────────────────────────────────────────────────────────────────────

## test: run all unit and doc tests (no I/O needed)
test:
	cd $(BACKEND_DIR) && cargo test --workspace

## test-integration: run integration tests (requires Docker for testcontainers)
test-integration:
	cd $(BACKEND_DIR) && cargo test --workspace -- --include-ignored

# ── Containers ─────────────────────────────────────────────────────────────────

## up: build and start the full stack (frontend, backend, postgres) on :8088
up:
	docker compose up --build -d
	@echo "TestModeller is on http://localhost:8088"

## down: stop the stack, keeping the database volume
down:
	docker compose down

## down-volumes: stop the stack and DELETE the database volume
down-volumes:
	docker compose down --volumes

## logs: follow logs from all containers
logs:
	docker compose logs -f

## ps: show container status and health
ps:
	docker compose ps

## docker-build: build both images without starting anything
docker-build:
	docker compose build

# ── Clean ──────────────────────────────────────────────────────────────────────

## clean: remove build artefacts
clean:
	cd $(BACKEND_DIR) && cargo clean

# ── Help ───────────────────────────────────────────────────────────────────────

help:
	@grep -E '^## ' Makefile | sed 's/^## //'

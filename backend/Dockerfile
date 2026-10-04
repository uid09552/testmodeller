# ── Build ─────────────────────────────────────────────────────────────────────
# Dependencies are compiled in their own layer first, so editing crate sources
# does not invalidate the (slow) dependency build.
FROM rust:1.94-slim-bookworm AS builder

WORKDIR /build

RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config \
 && rm -rf /var/lib/apt/lists/*

# 1. Manifests only, with stub sources, to cache the dependency build.
COPY Cargo.toml Cargo.lock ./
COPY crates/domain/Cargo.toml     crates/domain/
COPY crates/generation/Cargo.toml crates/generation/
COPY crates/storage/Cargo.toml    crates/storage/
COPY crates/ai/Cargo.toml         crates/ai/
COPY crates/api/Cargo.toml        crates/api/

RUN mkdir -p crates/domain/src crates/generation/src crates/storage/src \
             crates/ai/src crates/api/src \
 && echo 'pub fn stub() {}' | tee crates/domain/src/lib.rs \
      crates/generation/src/lib.rs crates/storage/src/lib.rs \
      crates/ai/src/lib.rs crates/api/src/lib.rs > /dev/null \
 && echo 'fn main() {}' > crates/api/src/main.rs \
 && cargo build --release --bin testmodeller \
 && rm -rf crates/*/src

# 2. Real sources. `sqlx::migrate!` reads the migrations directory at compile
#    time, so it has to be present here. There are no compile-time query
#    macros, so no database is needed to build.
COPY crates ./crates

RUN touch crates/*/src/lib.rs crates/api/src/main.rs \
 && cargo build --release --bin testmodeller \
 && strip target/release/testmodeller

# ── Runtime ───────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --create-home --uid 10001 app

COPY --from=builder /build/target/release/testmodeller /usr/local/bin/testmodeller

USER app
WORKDIR /home/app

# Containers must listen on all interfaces; the binary defaults to 127.0.0.1.
ENV TM_BIND_ADDR=0.0.0.0:8080
EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=3s --start-period=20s --retries=5 \
  CMD curl -fsS http://127.0.0.1:8080/api/v1/health || exit 1

# TM_DATABASE_URL must be supplied. --dev-mode is deliberately not used here:
# it starts a throwaway database via testcontainers and needs a Docker socket.
ENTRYPOINT ["/usr/local/bin/testmodeller"]

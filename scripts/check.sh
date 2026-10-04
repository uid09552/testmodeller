#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"

(cd "$root/backend" && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test)

if [ -f "$root/frontend/package.json" ]; then
  (cd "$root/frontend" && npm run lint && npm test -- --watch=false && npm run build)
fi

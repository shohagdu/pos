#!/usr/bin/env bash
# Builds the cPanel upload package: static (musl) binaries, the SPA and the
# Node launcher, zipped as dist/pos-cpanel.zip. See infra/cpanel/README.md.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TARGET=x86_64-unknown-linux-musl
OUT="$ROOT/dist/pos-cpanel"

rustup target add "$TARGET" >/dev/null
# ring compiles a little C; without musl-tools, the system gcc works for it.
command -v musl-gcc >/dev/null || export CC_x86_64_unknown_linux_musl=gcc

(cd "$ROOT/backend" && cargo build --release --target "$TARGET" -p pos-api -p pos-worker)
(cd "$ROOT/frontend" && npm ci && npm run build)

rm -rf "$OUT" "$OUT.zip"
mkdir -p "$OUT/bin" "$OUT/storage" "$OUT/logs"
cp "$ROOT/backend/target/$TARGET/release/pos-api" "$ROOT/backend/target/$TARGET/release/pos-worker" "$OUT/bin/"
cp -r "$ROOT/frontend/dist" "$OUT/public"
find "$OUT/public" -name '*.map' -delete
cp "$ROOT/infra/cpanel/app.js" "$OUT/"
cp "$ROOT/infra/cpanel/env.cpanel.example" "$OUT/env.example"
(cd "$OUT" && zip -qr ../pos-cpanel.zip .)
echo "built $ROOT/dist/pos-cpanel.zip ($(du -h "$ROOT/dist/pos-cpanel.zip" | cut -f1))"

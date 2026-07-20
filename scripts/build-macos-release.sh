#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="$ROOT/dist/jst-macos-universal"
TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)

export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"

rustup target add "${TARGETS[@]}"
for target in "${TARGETS[@]}"; do
    cargo build \
        --locked \
        --release \
        --package jst-cli \
        --target "$target" \
        --manifest-path "$ROOT/Cargo.toml"
done

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"
lipo -create \
    "$ROOT/target/aarch64-apple-darwin/release/jst" \
    "$ROOT/target/x86_64-apple-darwin/release/jst" \
    -output "$OUTPUT_DIR/jst"
chmod 755 "$OUTPUT_DIR/jst"
cp "$ROOT/LICENSE" "$OUTPUT_DIR/LICENSE"

lipo -info "$OUTPUT_DIR/jst"
"$OUTPUT_DIR/jst" --version
echo "$OUTPUT_DIR"

#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE="$ROOT/crates/apple-intelligence/main.swift"
OUTPUT="${1:?pass the helper output path}"
TARGETS=(arm64 x86_64)

[[ -f "$SOURCE" ]] || {
    echo "error: $SOURCE not found" >&2
    exit 1
}

TEMP_DIR="$(mktemp -d)"
trap 'rm -rf -- "$TEMP_DIR"' EXIT

for arch in "${TARGETS[@]}"; do
    xcrun --sdk macosx swiftc \
        -O \
        -parse-as-library \
        -target "$arch-apple-macos27.0" \
        "$SOURCE" \
        -o "$TEMP_DIR/jst-apple-intelligence-$arch"
done

mkdir -p "$(dirname "$OUTPUT")"
lipo -create \
    "$TEMP_DIR/jst-apple-intelligence-arm64" \
    "$TEMP_DIR/jst-apple-intelligence-x86_64" \
    -output "$OUTPUT"
chmod 755 "$OUTPUT"

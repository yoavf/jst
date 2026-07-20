#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="$ROOT/dist/jst-macos-universal"
BINARY="$OUTPUT_DIR/jst"
ARCHIVE_NAME="${1:-jst-macos-universal.zip}"
ARCHIVE="$ROOT/dist/$ARCHIVE_NAME"
CHECKSUM="$ARCHIVE.sha256"

if [[ "$ARCHIVE_NAME" != "$(basename "$ARCHIVE_NAME")" || "$ARCHIVE_NAME" != *.zip ]]; then
    echo "error: archive name must be a .zip filename without a path" >&2
    exit 1
fi

[ -x "$BINARY" ] || {
    echo "error: $BINARY not found — run scripts/build-macos-release.sh first" >&2
    exit 1
}
: "${AC_API_KEY_ID:?AC_API_KEY_ID is required}"
: "${AC_API_ISSUER_ID:?AC_API_ISSUER_ID is required}"
: "${AC_API_KEY_PATH:?AC_API_KEY_PATH is required}"
: "${SIGNING_TEAM_ID:=5PLGCB6G83}"

if [ -z "${SIGNING_IDENTITY:-}" ]; then
    SIGNING_IDENTITY="$(security find-identity -v -p codesigning | awk -F'"' -v team="$SIGNING_TEAM_ID" '$2 ~ /^Developer ID Application:/ && $2 ~ "\\(" team "\\)$" {print $2; exit}')"
    [ -n "$SIGNING_IDENTITY" ] || {
        echo "error: no Developer ID Application identity in keychain" >&2
        exit 1
    }
fi

echo "Signing as: $SIGNING_IDENTITY"
codesign --force --options runtime --timestamp --sign "$SIGNING_IDENTITY" "$BINARY"
codesign --verify --strict --verbose=2 "$BINARY"
codesign -dv --verbose=4 "$BINARY" 2>&1 | grep -Fq "TeamIdentifier=$SIGNING_TEAM_ID"

rm -f "$ARCHIVE" "$CHECKSUM"
/usr/bin/ditto -c -k --keepParent "$OUTPUT_DIR" "$ARCHIVE"

VERIFY_DIR="$(mktemp -d)"
trap 'rm -rf -- "$VERIFY_DIR"' EXIT
/usr/bin/ditto -x -k "$ARCHIVE" "$VERIFY_DIR"
VERIFIED_BINARY="$VERIFY_DIR/jst-macos-universal/jst"
codesign --verify --strict --verbose=2 "$VERIFIED_BINARY"
codesign -dv --verbose=4 "$VERIFIED_BINARY" 2>&1 | grep -Fq "TeamIdentifier=$SIGNING_TEAM_ID"
"$VERIFIED_BINARY" --version

xcrun notarytool submit "$ARCHIVE" \
    --key "$AC_API_KEY_PATH" \
    --key-id "$AC_API_KEY_ID" \
    --issuer "$AC_API_ISSUER_ID" \
    --wait

(cd "$ROOT/dist" && shasum -a 256 "$ARCHIVE_NAME" > "$(basename "$CHECKSUM")")

echo "Done: $ARCHIVE"
echo "Note: Apple does not support stapling tickets to ZIP archives or standalone executables."

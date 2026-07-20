#!/usr/bin/env bash
# Render the Homebrew formula for a jst release.
# Usage: render-homebrew-formula.sh <version> <sha256> [output-file]
#   version: release version without the "v" prefix (e.g. 0.0.1)
#   sha256:  SHA-256 of jst-v<version>-macos-universal.zip
#   output-file: defaults to stdout
set -euo pipefail

VERSION="${1:?version is required (e.g. 0.0.1)}"
SHA256="${2:?sha256 is required}"
OUTPUT="${3:--}"

if [[ "$VERSION" == v* ]]; then
    echo "error: version must not include the v prefix" >&2
    exit 1
fi

if [[ ! "$SHA256" =~ ^[0-9a-f]{64}$ ]]; then
    echo "error: sha256 must be 64 lowercase hex characters" >&2
    exit 1
fi

render() {
    cat <<EOF
class Jst < Formula
  desc "Run shell commands from natural-language requests"
  homepage "https://github.com/yoavf/jst"
  url "https://github.com/yoavf/jst/releases/download/v${VERSION}/jst-v${VERSION}-macos-universal.zip"
  sha256 "${SHA256}"
  license "MIT"

  depends_on :macos

  def install
    bin.install "jst"
  end

  test do
    system bin/"jst", "--version"
  end
end
EOF
}

if [[ "$OUTPUT" == "-" ]]; then
    render
else
    render > "$OUTPUT"
fi

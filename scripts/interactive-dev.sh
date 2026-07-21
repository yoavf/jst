#!/bin/sh

set -eu

if [ "$#" -eq 0 ]; then
    echo "usage: ./scripts/interactive-dev.sh <request>" >&2
    exit 2
fi

repo_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
port=${JST_EXPLAIN_PORT:-18181}
llm_url=${LLM_API_URL:-https://openrouter.ai/api/v1/chat/completions}
llm_model=${LLM_MODEL:-ibm-granite/granite-4.1-8b}
llm_key=${LLM_API_KEY:-${OPENROUTER_API_KEY:-}}
server_log=$(mktemp "${TMPDIR:-/tmp}/jst-explain-server.XXXXXX")
server_pid=""
terminal_state=""

cleanup() {
    if [ -n "$terminal_state" ]; then
        stty "$terminal_state" 2>/dev/null || true
    fi
    if [ -n "$server_pid" ] && kill -0 "$server_pid" 2>/dev/null; then
        kill "$server_pid" 2>/dev/null || true
        wait "$server_pid" 2>/dev/null || true
    fi
    rm -f "$server_log"
}
trap cleanup EXIT HUP INT TERM

if [ "$llm_url" = "https://openrouter.ai/api/v1/chat/completions" ] && [ -z "$llm_key" ]; then
    if [ ! -t 0 ]; then
        echo "OPENROUTER_API_KEY is not set and no terminal is available to ask for it." >&2
        exit 2
    fi

    printf "OpenRouter API key (input hidden): " >&2
    terminal_state=$(stty -g)
    stty -echo
    if ! IFS= read -r llm_key; then
        echo >&2
        exit 2
    fi
    stty "$terminal_state"
    terminal_state=""
    echo >&2

    if [ -z "$llm_key" ]; then
        echo "No API key entered." >&2
        exit 2
    fi
fi

if curl --silent --fail "http://127.0.0.1:${port}/health" >/dev/null 2>&1; then
    echo "port ${port} is already serving HTTP; choose another with JST_EXPLAIN_PORT." >&2
    exit 2
fi

echo "Building the worktree CLI and server..." >&2
(cd "$repo_root" && cargo build --quiet -p jst-server -p jst-cli)

echo "Starting isolated JST server on 127.0.0.1:${port}..." >&2
PORT="$port" \
    LLM_API_URL="$llm_url" \
    LLM_API_KEY="$llm_key" \
    LLM_MODEL="$llm_model" \
    MONTHLY_REQUEST_LIMIT=0 \
    REQUESTS_PER_MINUTE=0 \
    DAILY_REQUESTS_PER_IP=0 \
    GLOBAL_DAILY_REQUEST_LIMIT=0 \
    "$repo_root/target/debug/jst-server" >"$server_log" 2>&1 &
server_pid=$!

attempt=0
while ! curl --silent --fail "http://127.0.0.1:${port}/health" >/dev/null 2>&1; do
    if ! kill -0 "$server_pid" 2>/dev/null; then
        echo "The local JST server failed to start:" >&2
        sed -n '1,120p' "$server_log" >&2
        exit 1
    fi
    attempt=$((attempt + 1))
    if [ "$attempt" -ge 120 ]; then
        echo "Timed out waiting for the local JST server." >&2
        sed -n '1,120p' "$server_log" >&2
        exit 1
    fi
    sleep 1
done

echo "Running the worktree CLI. Nothing executes until you approve it." >&2
JST_API_URL="http://127.0.0.1:${port}/translate" \
    "$repo_root/target/debug/jst" --interactive "$@"

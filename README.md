# jst

Run shell commands from natural-language requests:

```sh
jst find all files bigger than 500 mb in ~/downloads
```

## Install

### Homebrew

```sh
brew install yoavf/tap/jst
```

### Manual

Signed and notarized universal macOS binaries are published with each GitHub
release. Download the ZIP for the latest release, then install `jst` somewhere
on your `PATH`, for example:

```sh
install -m 755 jst-macos-universal/jst ~/.local/bin/jst
```

The archive includes both Apple Silicon and Intel support. SHA-256 checksums are
published beside each release artifact.

## Use

JST prints the generated command and immediately executes it. Commands that
match the local destructive-command denylist, or whose model-described effects
indicate deletion, privilege use, process changes, software installation,
remote changes, or downloaded-code execution, require confirmation.

```text
→ find /Users/me/downloads -type f -size +500M
```

Use `--yolo` to skip all safety confirmations:

```sh
jst --yolo remove all stopped docker containers
```

Use `--dry` to print a generated command and exit without running it:

```sh
jst --dry show the current directory
```

### Review and refine

Use `-i` or `--interactive` to inspect and refine a command before anything
runs:

```console
$ jst -i show me the 10 largest files in this folder

→ du -ah . | sort -hr | head -n 10

Run it?  [y]es  [n]o  [w]hy  [a]sk AI  [e]dit  › w

  du -ah .      measure every entry (“files in this folder”)
  | sort -hr    order sizes largest first (“largest”)
  | head -n 10  keep the first ten results (“show me the 10”)

  Effects: reads local data.

Run it?  [y]es  [n]o  [w]hy  [a]sk AI  [e]dit  › a

✦ What should AI change? files only, not directories

→ find . -type f -exec du -h {} + | sort -hr | head -n 10

Run it?  [y]es  [n]o  [w]hy  [a]sk AI  [e]dit  ›
```

Each change is translated again with the original request and current command
as context, and its effects are recalculated before the revised command is
shown. Choose `e` to edit the current command inline, prefilled with the cursor
at the end. Arrow keys, Home, End, Delete, and Backspace work normally. Enter
approves the edited command for execution. Manual edits stay entirely local and
never call AI. If an edit matches JST's local destructive-command denylist, JST
shows the warning and asks again instead of running silently.

Pressing Escape while entering an AI change or editing the command discards
that draft and returns to the action menu. Empty input, `n`, and `q` abort
safely.

Interactive mode asks the model for detailed explanation metadata up front, so
choosing `w` does not require another request. `--interactive` and `--dry`
cannot be combined with `--yolo`.

If a server does not support structured explanations, JST falls back to the
standalone prose explanation.

## Server

By default, the CLI sends translation requests to the hosted JST server. The
proxy keeps provider credentials out of the distributed binary and lets JST
change models, prompts, and provider settings without requiring users to
install a new CLI release. The generated command is still checked locally
before execution. The complete proxy source lives in
[`crates/server`](crates/server); JST is open source end to end.

The hosted server currently applies these safeguards:

- 1,000 translations per anonymous installation in a fixed 30-day window.
- 20 translations per minute per client IP at the Fly proxy.
- 100 translations per client IP and 5,000 globally per fixed 24-hour window.
- A 32-request concurrency cap, 512-byte prompts and revision instructions,
  8 KiB request bodies, bounded model outputs, and provider timeouts.
- Strict OS and shell metadata validation, with provider details hidden from
  clients and provider outages identified separately from JST server errors.
- Rate-limit response headers for each active quota.

The CLI creates a random installation ID in its config directory and sends it
with translation requests. The server stores only an in-memory hash of that ID;
older clients fall back to a Fly-provided IP address. This is a best-effort
spending brake, not identity: deleting the ID bypasses it, and counters reset
when the server restarts or is redeployed.

You do not have to use the hosted proxy. The bundled server works with any
OpenAI-compatible chat-completions API. For example, using OpenRouter:

```sh
LLM_API_URL=https://openrouter.ai/api/v1/chat/completions \
LLM_API_KEY=... \
LLM_MODEL=microsoft/phi-4 \
LLM_FALLBACK_MODEL=google/gemma-4-26b-a4b-it \
cargo run --release -p jst-server
```

Then point the CLI at it:

```sh
JST_API_URL=http://127.0.0.1:8080/translate jst find large files
```

The server listens on `PORT` (default `8080`).
`MAX_CONCURRENT_TRANSLATIONS` optionally limits simultaneous provider calls.
`MONTHLY_REQUEST_LIMIT` controls the 30-day quota; set it to `0` to disable
anonymous usage tracking on your own server.
`REQUESTS_PER_MINUTE`, `DAILY_REQUESTS_PER_IP`, and
`GLOBAL_DAILY_REQUEST_LIMIT` control the short-term, daily client-IP, and global
daily limits. Each accepts `0` to disable it. The bundled implementation trusts
Fly's `Fly-Client-IP` header; self-hosters should only enable IP limits behind a
proxy that overwrites that header rather than accepting it from clients.
`LLM_API_KEY` is optional for local APIs that do not require authentication.
`LLM_FALLBACK_MODEL` optionally selects a model to try when `LLM_MODEL` fails.
Alternatively, `JST_API_URL` can point directly to any service implementing
JST's `/translate` JSON contract.

## Development

GitHub Actions runs formatting, build, tests, and Clippy on every pull request
and push to `main`.

```sh
cargo test --workspace
cargo build --workspace
```

The reusable [model benchmark](crates/server/examples/benchmark_models.md)
compares command generation, effect classification, parse reliability, and
latency directly against an OpenAI-compatible provider without touching hosted
JST usage statistics.

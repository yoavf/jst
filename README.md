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

Use `--dry` to require confirmation for every generated command, including
commands that do not trigger a safety warning:

```sh
jst --dry show the current directory
```

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
- A 32-request concurrency cap, 512-byte prompts, 2 KiB request bodies,
  256-token model outputs, and provider timeouts.
- Strict OS and shell metadata validation, with provider errors hidden from
  clients.
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
LLM_MODEL=ibm-granite/granite-4.1-8b \
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
Alternatively, `JST_API_URL` can point directly to any service implementing
JST's `/translate` JSON contract.

## Development

GitHub Actions runs formatting, build, tests, and Clippy on every pull request
and push to `main`.

```sh
cargo test --workspace
cargo build --workspace
```

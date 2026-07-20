# jst

Run shell commands from natural-language requests:

```sh
jst find all files bigger than 500 mb in ~/downloads
```

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

## Server

By default, the CLI sends translation requests to the hosted JST server. The
proxy keeps provider credentials out of the distributed binary and lets JST
change models, prompts, and provider settings without requiring users to
install a new CLI release. The generated command is still checked locally
before execution. The complete proxy source lives in
[`crates/server`](crates/server); JST is open source end to end.

The hosted server currently applies these safeguards:

- 1,000 translations per anonymous installation in a rolling 30-day window.
- A 32-request concurrency cap, bounded request and response sizes, and provider
  timeouts.
- Strict OS and shell metadata validation, with provider errors hidden from
  clients.
- `X-RateLimit-Limit` and `X-RateLimit-Remaining` response headers.

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
LLM_MODEL=google/gemini-2.5-flash-lite \
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

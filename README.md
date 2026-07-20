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
before execution.

You do not have to use the hosted proxy. Run the bundled OpenRouter server with
your own API key:

```sh
OPENROUTER_API_KEY=... \
OPENROUTER_MODEL=google/gemini-2.5-flash-lite \
cargo run --release -p jst-server
```

Then point the CLI at it:

```sh
JST_API_URL=http://127.0.0.1:8080/translate jst find large files
```

The server listens on `PORT` (default `8080`).
`MAX_CONCURRENT_TRANSLATIONS` optionally limits simultaneous provider calls.
The bundled server currently uses OpenRouter, but `JST_API_URL` can point to any
service implementing JST's `/translate` JSON contract, including a custom
adapter for Ollama or another local model.

## Development

GitHub Actions runs formatting, build, tests, and Clippy on every pull request
and push to `main`.

```sh
cargo test --workspace
cargo build --workspace
```

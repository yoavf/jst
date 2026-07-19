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

## Development

GitHub Actions runs formatting and workspace tests on every pull request and
push to `main`.

```sh
cargo test --workspace
cargo build --workspace
```

Run the API server locally with an OpenRouter key:

```sh
OPENROUTER_API_KEY=... OPENROUTER_MODEL=... cargo run -p jst-server
```

Benchmark the shortlisted OpenRouter models for latency, command quality, and
effect classification:

```sh
OPENROUTER_API_KEY=... cargo run -p jst-server --example benchmark_models
```

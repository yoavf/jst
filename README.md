# jst

Run shell commands from natural-language requests:

```sh
jst find all files bigger than 500 mb in ~/downloads
```

<p align="center">
  <img src="https://jst.sh/demos/clear-port-8080.gif" width="960" alt="JST safely clearing a process from port 8080">
</p>

<p align="center"><a href="https://jst.sh">See more demos or try JST in the browser sandbox</a></p>

## Install

### Homebrew

```sh
brew install yoavf/tap/jst
```

### Manual

Each GitHub release includes builds for:

- macOS (signed and notarized universal binary for Apple Silicon and Intel)
- Linux x86-64
- Windows x86-64

Download the archive for your platform and put `jst` (or `jst.exe` on Windows)
somewhere on your `PATH`. For example, on macOS or Linux:

```sh
install -m 755 jst-*/jst ~/.local/bin/jst
```

For WSL, use the Linux archive and install it inside your WSL distribution. Use
the Windows archive when running `jst` from native Command Prompt or PowerShell.

SHA-256 checksums are published beside every release artifact.

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

### Apple Intelligence (macOS 27 beta)

On macOS 27.0 beta or later, use the on-device Apple Intelligence model instead
of the hosted JST server. It is opt-in.

Apple mode keeps requests on your Mac and needs no API key, but it is not yet
reliable enough to replace hosted JST. In a 20-case macOS/zsh benchmark, Apple
passed 11/20 effect checks and only 4/20 were fully correct on manual review;
hosted Phi-4 and Gemma passed 19/20 and 20/20 respectively.

```sh
jst --provider apple --dry show the current directory
jst --provider apple -i find files larger than 500 MB
jst --provider apple --status
```

If you use it regularly, set the provider once in your shell configuration:

```sh
export JST_PROVIDER=apple
jst show the current directory
```

Use `--provider server` on an individual command to override that setting.

This option requires a Mac and Apple Intelligence configuration for which the
system model is available. `jst --provider apple --status` reports the model's
availability before any translation. The request, generated command, and
revision instructions stay on the Mac: this provider does not contact the JST
server, consume its quota, or use an API key.

On macOS 26 or earlier (and on non-macOS platforms), selecting Apple mode exits
before launching the helper with a clear macOS-27 requirement. Run JST without
the Apple provider, or use `--provider server`, to keep using the hosted model.

Mac release archives include a `jst-apple-intelligence` companion executable;
Linux and Windows archives do not. Homebrew installs it privately and wires it
up automatically. For a manual macOS install, keep the two archive executables
together in the same directory on your `PATH`:

```sh
install -m 755 jst-*/jst jst-*/jst-apple-intelligence ~/.local/bin/
```

The Rust CLI sends the companion a JSON request and receives a structured JSON
response; the companion calls Apple's `FoundationModels` framework directly.
This keeps the beta framework out of Rust and avoids maintaining Swift
bindings. The hosted JST server is available on every supported platform.

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

### Server status

Use `--status` to check the configured JST server without calling the model:

```console
$ jst --status
Server: ok
Primary model: provider/primary-model
Fallback model: provider/fallback-model
Calls today: 123
Calls all time: 4567
```

The usage totals are anonymous aggregates and display as unavailable when the
server's stats store is disabled or temporarily unreachable.

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
- A 256-request concurrency cap, 512-byte prompts and revision instructions,
  8 KiB request bodies, bounded model outputs, and a five-second timeout for
  each primary or fallback model attempt.
- Strict OS and shell metadata validation, with provider details hidden from
  clients and provider outages identified separately from JST server errors.
- Rate-limit response headers for each active quota.

### Browser demo

The website’s “try it now” flow uses the real hosted translator through a
separate `POST /demo` endpoint, then executes the returned command entirely in
the browser with pinned uutils WebAssembly binaries and an in-memory WASI
filesystem. The browser runtime is deliberately narrower than the CLI:

- More than 50 Rust coreutils plus `find`, `grep`, `diff`, and `cmp` may run.
  File utilities can modify only the disposable in-memory workspace. Simple
  pipelines and bounded input redirection are supported; output redirects,
  substitutions, loops, nested shells, process-spawning flags, and other shell
  control syntax are rejected by the server, page, and sandbox.
- Every terminal session gets a small fake filesystem. Its in-memory state
  persists between commands so a directory created with `mkdir` can be listed
  afterward. Reset, page exit, timeout, or excessive output destroys it. No
  host files, processes, environment variables, or credentials are mounted.
- The WASI shim exposes no sockets or host syscalls. Each command is stopped
  after six seconds or 32 KiB of output.
- Output is rendered as text after terminal controls and bidirectional display
  controls are escaped.

The website stores a random browser UUID locally and sends it only as a soft
quota identifier. The hosted service applies independent rolling browser,
per-minute, client IP, and global limits. Exact quotas are intentionally not
shown in the terminal. IP and global caps remain the spending backstop because
a user can clear browser storage and receive a new UUID.

`DEMO_ALLOWED_ORIGINS` is a comma-separated exact origin allowlist for the
browser endpoint. `DEMO_MONTHLY_REQUEST_LIMIT`,
`DEMO_REQUESTS_PER_MINUTE`, `DEMO_DAILY_REQUESTS_PER_IP`, and
`DEMO_GLOBAL_DAILY_REQUEST_LIMIT` configure its independent quota namespace.
Each numeric limit accepts `0` to disable it.

`DEMO_LLM_API_KEY` configures a provider credential used only by `/demo`;
`DEMO_OPENROUTER_API_KEY` is accepted as an alias. The demo never falls back to
`LLM_API_KEY` or `OPENROUTER_API_KEY`, so the browser and CLI traffic can be
tracked and revoked independently. If the demo key is absent or rejected by
the provider, `/demo` returns a generic temporary-unavailability response
without exposing provider details. Set `DEMO_LLM_API_KEY` to an explicitly
empty value when a self-hosted provider intentionally requires no
authentication.

The CLI creates a random installation ID in its config directory and sends it
with translation requests. The server stores only a hash of that ID; older
clients fall back to a Fly-provided IP address. When
`UPSTASH_REDIS_REST_URL` and `UPSTASH_REDIS_REST_TOKEN` are set, quota counters
are atomically enforced in the shared Redis store across every server instance.
Without Upstash, the server falls back to per-process in-memory counters that
reset when the process restarts. This is a best-effort spending brake, not
identity: deleting the installation ID bypasses it.

You do not have to use the hosted proxy. The bundled server works with any
OpenAI-compatible chat-completions API. For example, using OpenRouter:

```sh
LLM_API_URL=https://openrouter.ai/api/v1/chat/completions \
LLM_API_KEY=... \
DEMO_LLM_API_KEY=... \
LLM_MODEL=google/gemma-4-26b-a4b-it \
LLM_FALLBACK_MODEL=microsoft/phi-4 \
cargo run --release -p jst-server
```

Then point the CLI at it:

```sh
JST_API_URL=http://127.0.0.1:8080/translate jst find large files
```

`jst --status` derives the sibling `/status` endpoint from `JST_API_URL`. Set
`JST_STATUS_URL` when a custom deployment exposes status at a different URL.

The server listens on `PORT` (default `8080`).
`MAX_CONCURRENT_TRANSLATIONS` optionally limits simultaneous provider calls.
`MONTHLY_REQUEST_LIMIT` controls the 30-day quota; set it to `0` to disable
anonymous usage tracking on your own server.
`REQUESTS_PER_MINUTE`, `DAILY_REQUESTS_PER_IP`, and
`GLOBAL_DAILY_REQUEST_LIMIT` control the short-term, daily client-IP, and global
daily limits. Each accepts `0` to disable it. The bundled implementation trusts
Fly's `Fly-Client-IP` header; self-hosters should only enable IP limits behind a
proxy that overwrites that header rather than accepting it from clients.
Configure `UPSTASH_REDIS_REST_URL` and `UPSTASH_REDIS_REST_TOKEN` to share both
rate-limit counters and aggregate usage statistics across server instances.
The `/stats` snapshot includes browser-toolbox miss totals and the most common
missing base commands. Prompts, arguments, paths, and complete generated
commands are never retained.
`LLM_API_KEY` is optional for local APIs that do not require authentication.
`LLM_FALLBACK_MODEL` optionally selects a model to try when `LLM_MODEL` fails.
Alternatively, `JST_API_URL` can point directly to any service implementing
JST's `/translate` JSON contract.

## Similar projects

There are several thoughtful takes on natural-language shell interfaces. If
JST is useful to you, these may be worth exploring too:

- [ask](https://github.com/ykushch/ask) — local, Ollama-powered command
  translation with project context and an interactive mode.
- [AI CLI](https://github.com/kriserickson/ai-cli) — a multi-provider command
  translator with one-shot and interactive modes and an execution safety
  policy.
- [zsh-ai](https://github.com/matheusml/zsh-ai) — inserts a generated command
  into the current zsh prompt so it can be reviewed and edited before running.
- [ShellSage](https://github.com/AnswerDotAI/shell_sage) — a tmux-aware
  assistant that uses terminal history and piped output as context.
- [NatShell](https://github.com/Barent/natshell) — a local-first agentic shell
  TUI with bundled-model and remote-provider support.
- [AISH](https://github.com/AI-Shell-Team/aish) — a full PTY shell with
  natural-language operations, command explanations, risk levels, and an
  optional sandbox pre-run.

## Development

GitHub Actions runs formatting, build, tests, and Clippy on every pull request
and push to `main`.

```sh
cargo test --workspace
cargo build --workspace
```

The site is served from `docs/`. Its browser sandbox bundle is generated from
`site/`; the pinned uutils binaries and their checksums are documented in
`docs/assets/uutils/ATTRIBUTION.md`:

```sh
npm ci
npm run build:demo
npm run dev:demo
```

The production host applies the COOP/COEP headers in `docs/_headers` as
additional browser isolation, even though this single-threaded WASI runtime
does not require `SharedArrayBuffer`.

The reusable [model benchmark](crates/server/examples/benchmark_models.md)
compares command generation, effect classification, parse reliability, and
latency directly against an OpenAI-compatible provider without touching hosted
JST usage statistics.

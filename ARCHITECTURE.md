# jst — Repository Architecture

## Overview

JST is a Cargo workspace with three crates:

- `jst-cli` — command translation, interactive refinement, safety checks, and
  execution.
- `jst-server` — a thin proxy for OpenAI-compatible LLM APIs.
- `jst-shared` — request types, response types, effect policy, and prompting.

## Request Flow

```text
jst natural language request
  → POST /translate
  → OpenAI-compatible LLM API
  → command + concrete effect description + optional semantic parts
  → local denylist OR dangerous model effects
  → optional interactive session: explain, revise, manually replace,
    approve, or abort
      ↳ revisions return to POST /translate with original request,
        current command, and requested change
  → user's shell
```

The model describes concrete effects rather than assigning an abstract risk
score. The CLI decides whether those effects require confirmation. A model
response can add a warning but cannot suppress a warning from the local
denylist.

Explain requests ask the same model call for a bounded list of command parts.
Part fragments must concatenate exactly to the executable command, and source
phrases must occur in the user's request. The server sanitizes this metadata and
the CLI validates it again before rendering. Invalid explanation metadata is
discarded and never affects command execution or safety decisions.

Interactive mode requests explanation metadata up front so choosing `w` is
instantaneous. Choosing `a` sends structured revision context rather than
concatenating instructions into the original prompt. The model must return a
complete replacement command and recalculate its effects. The replacement goes
through the same server validation, local denylist, terminal-safety checks, and
explicit approval loop as the initial command.

Choosing `e` opens a prefilled inline editor with the cursor at the end. Enter
counts as execution approval and the edit remains entirely local: it is never
sent to the server or model. Safe edits run immediately; edits that match the
local destructive-command denylist return to the approval loop with a warning.
`--dry` does not enter this loop; it prints the initial translation and exits
before warnings, confirmation, or execution.

The server crate is the production proxy: it owns provider credentials,
validates and bounds requests and responses, limits concurrent provider calls,
reuses upstream connections, and returns no provider error details to clients.
Fly keeps one machine warm to avoid cold-start latency and deploys only after
CI passes.

## Anonymous Gating

The CLI creates a random installation ID and stores it in its config directory.
The proxy hashes that value in memory and allows 1,000 requests per rolling
30-day window. A separate limiter allows 20 requests per minute per
Fly-provided client IP. Older clients fall back to that address for the monthly
limit as well. Tracked fingerprints are bounded to prevent unbounded memory
growth.

This is intentionally a soft, instance-local limit. It resets after a process
restart or deployment and can be bypassed by deleting the installation ID.
Durable enforcement would require a shared TTL store; strict per-person
enforcement requires identity, payment, or platform attestation. The IP limiter
depends on a trusted reverse proxy overwriting `Fly-Client-IP`.

## Anonymous Usage Stats

The server keeps aggregate counters — total translations, a histogram of base
command names (`find`, `git`, …), and per-day totals for a 30-day trend — and
never stores request input, full commands, arguments, or installation
identifiers. Each machine buffers counts in memory and flushes them about once
a minute (and once on shutdown) to a shared serverless Redis over its REST
API, so any number of machines and regions contribute to the same totals
without database replication. Day keys expire after 40 days. `GET /stats`
returns the cached snapshot (60-second cache, CORS-enabled) powering the stats
section on the public website. Stats are disabled unless
`UPSTASH_REDIS_REST_URL` and `UPSTASH_REDIS_REST_TOKEN` are set, and count
successful translations only.

## Workspace

```text
crates/cli/src/main.rs       argument parsing, API calls, interactive loop, execution
crates/cli/src/installation.rs  anonymous installation ID persistence
crates/cli/src/safety.rs     deterministic destructive-command denylist
crates/server/src/main.rs    HTTP server and routes
crates/server/src/openai_compatible.rs  OpenAI-compatible request handling
crates/server/src/rate_limit.rs  bounded rolling-window usage limits
crates/server/src/stats.rs     buffered anonymous usage counters and /stats
crates/shared/src/types.rs   API contract and model-effect policy
crates/shared/src/prompt.rs  model instructions and output schema
```

## Operations

```sh
cargo test --workspace
cargo build --workspace
LLM_API_URL=... LLM_API_KEY=... LLM_MODEL=... cargo run -p jst-server
JST_API_URL=http://localhost:8080/translate cargo run -p jst-cli -- pwd
```

The CLI contains no provider credentials. Release binaries can be built and
signed as ordinary Rust executables.

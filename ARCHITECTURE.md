# jst — Repository Architecture

## Overview

JST is a Cargo workspace with three crates:

- `jst-cli` — one-shot command translation, safety checks, and execution.
- `jst-server` — a thin OpenRouter proxy that owns the API key.
- `jst-shared` — request types, response types, effect policy, and prompting.

## Request Flow

```text
jst natural language request
  → POST /translate
  → OpenRouter model
  → command + concrete effect description
  → local denylist OR dangerous model effects
  → optional confirmation
  → user's shell
```

The model describes concrete effects rather than assigning an abstract risk
score. The CLI decides whether those effects require confirmation. A model
response can add a warning but cannot suppress a warning from the local
denylist.

## Workspace

```text
crates/cli/src/main.rs       argument parsing, API call, confirmation, execution
crates/cli/src/safety.rs     deterministic destructive-command denylist
crates/server/src/main.rs    HTTP server and routes
crates/server/src/openrouter.rs  OpenRouter request and response handling
crates/shared/src/types.rs   API contract and model-effect policy
crates/shared/src/prompt.rs  model instructions and output schema
```

## Operations

```sh
cargo test --workspace
cargo build --workspace
OPENROUTER_API_KEY=... OPENROUTER_MODEL=... cargo run -p jst-server
```

The CLI contains no provider credentials. Release binaries can be built and
signed as ordinary Rust executables.

# jst — Repository Architecture

## Overview

Monorepo using a Cargo workspace. Three crates: the CLI binary users install, the API proxy server we deploy, and a shared library that keeps both in sync.

Both CLI and server are open source to inspire confidence — this tool executes shell commands, people should be able to see exactly what it does.

## Workspace Structure

```
jst/
  Cargo.toml                    # workspace root
  README.md
  PROJECT_BRIEF.md
  LICENSE

  crates/
    cli/                        # the binary users install (published as `jst`)
      Cargo.toml
      src/
        main.rs                 # CLI entry, arg parsing (clap)
        shell.rs                # interactive input loop — crossterm raw mode, rendering, key handling
        translator.rs           # sends input to jst API proxy (or BYOK provider in future)
        config.rs               # ~/.jst/config.toml loading/saving
        context.rs              # project context auto-detection + .jst.ctx parsing

    server/                     # the API proxy we deploy (Fly.io / Cloudflare)
      Cargo.toml
      src/
        main.rs                 # server entry, config loading, startup
        routes.rs               # POST /translate endpoint
        ratelimit.rs            # device hash tracking, daily counters, tier enforcement
        device_auth.rs          # hardware ID validation, GitHub OAuth device flow
        openrouter.rs           # upstream OpenRouter API client (holds the API key)

    shared/                     # library crate used by both CLI and server
      Cargo.toml
      src/
        lib.rs                  # re-exports
        types.rs                # request/response types (TranslateRequest, TranslateResponse, etc.)
        prompt.rs               # system prompt construction — lives here so both sides agree on format
        device.rs               # device hash generation (used by CLI to create, server to validate)
```

## Root Cargo.toml

```toml
[workspace]
members = ["crates/cli", "crates/server", "crates/shared"]
resolver = "2"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
```

## Crate Dependencies

### cli (`jst-cli`)

```toml
[package]
name = "jst-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "jst"
path = "src/main.rs"

[dependencies]
jst-shared = { path = "../shared" }
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
reqwest.workspace = true
crossterm = "0.28"
clap = { version = "4", features = ["derive"] }
dirs = "5"
toml = "0.8"
unicode-width = "0.2"
```

### server (`jst-server`)

```toml
[package]
name = "jst-server"
version = "0.1.0"
edition = "2021"

[dependencies]
jst-shared = { path = "../shared" }
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
reqwest.workspace = true
axum = "0.8"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }
tracing = "0.1"
tracing-subscriber = "0.3"
redis = { version = "0.27", features = ["tokio-comp"] }  # for rate limit counters
sha2 = "0.10"                                             # for device hash validation
```

### shared (`jst-shared`)

```toml
[package]
name = "jst-shared"
version = "0.1.0"
edition = "2021"

[dependencies]
serde.workspace = true
sha2 = "0.10"
```

## What Lives Where

### shared crate — the contract

This is the most important crate architecturally. It ensures the CLI and server never drift.

**`types.rs`** — the API contract:
```rust
pub struct TranslateRequest {
    pub input: String,           // natural language from user
    pub device_hash: String,     // hashed hardware ID
    pub context: Option<String>, // project context (auto-detected + .jst.ctx)
    pub os: String,              // "macos", "linux", "windows"
    pub shell: Option<String>,   // "zsh", "bash", "fish", etc.
}

pub struct TranslateResponse {
    pub command: String,              // translated shell command
    pub remaining_today: Option<u32>, // how many free translations left (anonymous tier)
}

pub struct ErrorResponse {
    pub error: String,
    pub code: String,         // "rate_limited", "auth_required", "upstream_error"
    pub upgrade_hint: Option<String>, // "Run `jst auth` for unlimited free tier"
}
```

**`prompt.rs`** — system prompt construction:
Both CLI (for future BYOK mode) and server need to build the same system prompt. Single source of truth here.

**`device.rs`** — device hash generation:
The CLI calls this to generate the hash. The server imports the same module to understand the format. The salt is a build-time constant or passed via config.

### cli crate — pure client

- No secrets, no API keys
- Reads hardware ID, hashes it, sends it with every request
- All inference goes through the jst API proxy (in v1)
- Handles: interactive shell, one-shot mode, config, context detection
- Future: BYOK mode calls providers directly, bypassing proxy

### server crate — thin proxy

- Holds the OpenRouter API key (via env var, never in code)
- Single endpoint: `POST /translate`
- Rate limiting: tracks `device_hash → daily_count` in Redis (or in-memory for dev)
- Tiers: anonymous (50/day by device hash), authenticated (200/day by GitHub user ID)
- GitHub OAuth device flow for authentication upgrade
- Forwards request to OpenRouter API with constructed system prompt
- Returns translated command + remaining quota

## Build & Run

```bash
# build everything
cargo build --workspace

# run CLI locally
cargo run -p jst-cli

# run server locally
OPENROUTER_API_KEY=... OPENROUTER_MODEL=qwen/qwen2.5-coder-7b-instruct cargo run -p jst-server

# test everything
cargo test --workspace

# release build for CLI (optimized, stripped)
cargo build -p jst-cli --release
```

## Deployment

- **CLI**: distributed via Homebrew, scoop/winget, GitHub Releases (prebuilt binaries per platform)
- **Server**: Docker container → Fly.io (or Cloudflare Worker if we go that route)
- **CI**: GitHub Actions — `cargo test --workspace` on every PR, cross-compile CLI binaries on release tags

## Why This Structure

- **Shared types = compile-time API contract.** Change a field in `TranslateRequest` and both sides fail to compile until they agree. No integration surprises.
- **Prompt logic in shared.** The system prompt is critical to output quality. Having it in one place means improvements benefit both the hosted service and future BYOK mode.
- **Open source everything.** Users can audit exactly what the CLI sends and what the server does with it. The only secret is the OpenRouter API key, which lives in the server's environment, never in code.
- **Independent deployment.** CLI and server have different release cadences. CLI ships when UX changes. Server ships when proxy logic changes. Workspace handles this naturally.

# jst — Project Brief

## One-liner

Real-time natural language to shell command translator. Type what you mean, get the command you need.

## The Problem

Developers (and especially those who work across many tools/frameworks) constantly forget exact CLI syntax. The current workflow is: try to remember → fail → Google/ChatGPT it → copy-paste → hope it's right. This breaks flow and wastes time.

## The Solution

`jst` is a CLI tool that sits between you and your shell. You type natural language, and it translates to the correct shell command in real-time as you type. When the command looks right, press Enter to execute. That's it.

```
› find all files larger than 1gb in home directory
  ⮑ find ~ -type f -size +1G

› compress the logs folder into a tarball
  ⮑ tar -czf logs.tar.gz logs/

› kill whatever is running on port 3000
  ⮑ lsof -ti:3000 | xargs kill -9
```

## How It Works

1. User launches `jst` — prompt changes to `›` to indicate you're inside the tool
2. User types natural language
3. After a short debounce (~300ms), the input is sent to the inference backend
4. The translated shell command appears below the input in real-time (cyan, with a ⮑ indicator)
5. A spinner shows while translation is in-flight
6. **Enter** → execute the command
7. **Esc / Ctrl+C** → cancel
8. **Tab** → accept translation into the input line for editing before running

### One-Shot Mode

```bash
jst find all png files modified today
# prints: find . -name "*.png" -mtime -1
```

## Technical Decisions

### Language: Rust

- Single binary distribution, no runtime dependencies
- Maximum performance for the input loop and rendering (crossterm/ratatui)
- Installs via Homebrew (macOS), scoop/winget (Windows)
- The bottleneck is always inference latency, never the CLI itself

### Model: Ministral 8B

- Fast, accurate for constrained code-generation tasks
- Served via Mistral API, proxied through our own backend
- Future: add BYOK (bring your own key) and local inference options

### Backend Architecture (v1 — simplified)

```
jst CLI binary → jst API proxy → Mistral API (Ministral 8B)
```

The **jst API proxy** is a lightweight service (Cloudflare Worker or Fly.io) that:
- Holds the Mistral API key (never shipped in the binary, never exposed to users)
- Receives `{input, device_hash, project_context}` from the CLI
- Checks rate limits and daily usage caps against the device hash
- Calls Mistral API with the system prompt + project context + user input
- Returns the translated command

This proxy is where all freemium logic lives: device fingerprint tracking, daily counters, and future GitHub auth. The CLI itself has zero secrets — it just hits our endpoint.

**Why not OpenRouter?** Not needed yet — we're only using one model. If we ever want model flexibility or provider fallbacks, we swap the proxy's upstream from Mistral to OpenRouter in one line. That's a backend change, invisible to users.

Future backends (not v1):
- BYOK: Anthropic, Groq, OpenAI, Mistral direct (user provides their own key, CLI calls provider directly, bypasses our proxy entirely)
- Local: Ollama, or bundled GGUF via llama.cpp bindings
- Self-hosted: for enterprise/air-gapped environments

### Freemium Model

**Goal: zero friction to start, pay/auth only when you hit a limit.**

- **Anonymous tier**: No signup required. Device identified by hashed hardware ID (`IOPlatformUUID` on macOS, `/etc/machine-id` on Linux, registry `MachineGuid` on Windows). Hash with a server-side salt so raw IDs never leave the device. **50 translations/day.**
- **Authenticated tier**: When user hits the limit, prompt: "Run `jst auth` for unlimited free tier with GitHub login." GitHub OAuth device code flow — opens browser, user clicks authorize, done. **200/day, free forever.**
- **BYOK tier** (future): Bring your own API key. Unlimited. No interaction with our infra.

**Anti-abuse notes:**
- Hardware ID only (no hostname/username/OS — those change too easily and weaken the fingerprint)
- Rate limiting by IP + device hash at the proxy level
- VMs/containers will regenerate machine-id — acceptable, falls through to "just auth with GitHub"
- Determined abuse is fine — the economics work even with some gaming. Ministral 8B via Mistral API costs fractions of a cent per translation

## Project Context Detection

`jst` auto-detects the project type in the current directory and feeds that as context to the model. This makes translations dramatically better for framework-specific commands.

### Auto-detected signals:
- **Laravel/PHP**: `composer.json` + `artisan` → knows about `php artisan` commands
- **Node.js/Next.js/Nuxt**: `package.json` → extracts available npm scripts
- **Rust**: `Cargo.toml` → cargo commands
- **Python/Django**: `requirements.txt`/`pyproject.toml` + `manage.py`
- **Ruby/Rails**: `Gemfile` + `bin/rails`
- **Go**: `go.mod`
- **Docker**: `Dockerfile`/`docker-compose.yml`
- **Makefile**: extracts available targets

### Custom context: `.jst.ctx`

Users can drop a `.jst.ctx` file in their project root with domain-specific hints:

```
This is a Laravel project for BrainPOP.
Content types include: bp_topics, bp_movies, bp_quizzes.
The publish command is: php artisan publish:content --content_type=TYPE --content_id=ID
```

This gets injected into the model's system prompt alongside the auto-detected context.

## UI/UX Details

- **Prompt indicator**: `›` (visually distinct from standard `$` or `%` prompts)
- **Translation display**: Below input line, prefixed with `⮑`, colored cyan
- **Spinner**: Braille animation (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`) while translating, dimmed
- **Error display**: Below translation line, red, prefixed with `⚠`
- **Alternate screen**: Opens in alternate terminal buffer so it doesn't pollute scroll history
- **Key bindings**: Enter (execute), Esc/Ctrl+C (quit), Tab (accept into input), Ctrl+U (clear), Ctrl+W (delete word), Up/Down (history), Home/End/Ctrl+A/Ctrl+E (cursor movement)

## System Prompt (core)

```
You are a CLI command translator. Convert natural language into executable shell commands.

Rules:
- Output ONLY the shell command, nothing else. No explanation, no markdown, no backticks.
- If the input is ambiguous, output the most likely intended command.
- If the input is already a valid command, output it as-is.
- For loops and multi-command operations, use proper shell syntax.
- Prefer common, portable commands when possible.
- If you truly cannot translate the input, output: # unable to translate
```

Plus dynamic project context appended per-request.

## Architecture (Rust)

```
src/
  main.rs          # CLI entry, arg parsing (clap)
  shell.rs         # Interactive input loop — crossterm raw mode, rendering, key handling
  translator.rs    # Trait-based translation — currently just MistralTranslator
  config.rs        # ~/.jst/config.toml loading/saving
  context.rs       # Project context auto-detection + .jst.ctx parsing
```

### Key crate dependencies:
- `crossterm` — terminal raw mode, cursor control, styled output
- `tokio` — async runtime for non-blocking inference calls
- `reqwest` — HTTP client for API calls
- `clap` — CLI argument parsing
- `serde` / `toml` — config serialization
- `dirs` — cross-platform home directory detection

## Distribution

- **macOS**: Homebrew tap (`brew install yoavf/tap/jst`)
- **Windows**: scoop or winget
- **Linux**: .deb/.rpm packages, or direct binary download
- **From source**: `cargo install --path .`

## Example Interactions

```
› run artisan publish content on bp_topics ids 1 to 75, 200-210, 223
  ⮑ for id in $(seq 1 75) $(seq 200 210) 223; do php artisan publish:content --content_type=bp_topics --content_id=$id; done

› show me what's eating disk space
  ⮑ du -sh /* 2>/dev/null | sort -rh | head -20

› list all docker containers including stopped ones
  ⮑ docker ps -a

› ssh into the staging server and tail the app logs
  ⮑ ssh staging 'tail -f /var/log/app.log'

› git squash the last 5 commits
  ⮑ git rebase -i HEAD~5
```

## What's NOT in v1

- Local/offline inference (Ollama, bundled models)
- BYOK (bring your own API key)
- Session context (remembering previous commands within a session)
- Tab completion cycling between alternative translations
- Plugin/extension system
- Fine-tuned model (start with prompt engineering, collect correction data organically)
- Usage dashboard / analytics

## Future: Fine-Tuning Data Collection

When users press Tab to edit a translation before running, or when they Esc and retype — that's implicit correction data. In the future (with opt-in consent), collect these pairs:

- Input: natural language
- Generated: initial translation
- Corrected: what the user actually ran

This becomes the fine-tuning dataset for a purpose-built tiny model.

## Open Questions

- Should `jst` maintain conversational context within a session? ("now do the same but for bp_lessons")
- Should one-shot mode auto-execute or just print? (Current: just print, pipe to `sh` to execute)
- Pricing for authenticated tier beyond free — is 200/day enough? Do we need a paid tier?
- Should the model see recent shell history for better context?

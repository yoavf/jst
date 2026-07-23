# Phi-4 prompt autoresearch

`prompt_autoresearch.rs` searches prompt structures against deterministic
command-quality assertions while keeping the model fixed. It is intended for
prompt changes, not model comparisons.

## Run

Place `OPENROUTER_API_KEY` in `.env`, then run:

```sh
cargo run --release -p jst-server --example prompt_autoresearch
```

The runner reads `.env` without printing secrets. It defaults to:

- model: `microsoft/phi-4`;
- endpoint: OpenRouter chat completions;
- concurrency: 12 requests;
- beam width: 4 candidates;
- search rounds: 5.

Optional environment variables:

| Variable | Purpose |
| --- | --- |
| `JST_AUTORESEARCH_MODEL` | Keep explicit when reproducing a model-specific result |
| `JST_AUTORESEARCH_CONCURRENCY` | Maximum parallel requests |
| `JST_AUTORESEARCH_BEAM_WIDTH` | Candidates retained between search rounds |
| `JST_AUTORESEARCH_ROUNDS` | Maximum mutation rounds |
| `JST_AUTORESEARCH_API_URL` | Alternate OpenAI-compatible endpoint |
| `JST_AUTORESEARCH_API_KEY` | Credential for an alternate endpoint |

The OpenRouter key is never sent to an alternate endpoint.

## Search and gates

The fixed `baseline-v0.3.0` prompt is compared with mutations across:

- instruction ordering;
- environment placement;
- generic, narrow-safety, and detailed rule sets;
- zero-shot, positive, and contrastive examples;
- direct versus silent-checklist instructions;
- plain, labeled, and JSON user context.

Every candidate/case pair runs concurrently up to the configured limit.
Candidates are ranked by fully passing cases, passed assertions, parse
reliability, and then prompt length. Perfect training candidates become
finalists; held-out cases choose the shortest finalist that passes. The selected
prompt must then pass two complete stability repeats.

The graders inspect generated commands and effect metadata rather than trusting
the model's `matches_request`. Cases cover issues 30–34 plus ordinary commands.
Paired macOS and Linux cases send identical English requests and require
environment-appropriate output.

Generated audit artifacts are written under the ignored directory
`target/prompt-autoresearch/`:

- `latest.md` contains rankings and every held-out/stability command;
- `winning-system-prompt.txt` contains the selected prompt for one sample
  environment.

## July 23, 2026 result

The selected `rules-tail-safety-positive-direct-plain` prompt uses:

- ordered core requirements;
- two narrow safety rules;
- positive examples selected by target OS;
- plain initial requests and labeled revision sections;
- the target environment at the end of the system message.

| Gate | Phi-4 result |
| --- | ---: |
| v0.3.0 baseline training | 5/14 cases |
| selected prompt training | 14/14 cases, 58/58 assertions |
| held-out | 8/8 cases, 35/35 assertions |
| stability repeat 1 | 22/22 cases, 93/93 assertions |
| stability repeat 2 | 22/22 cases, 93/93 assertions |

For the same held-out request, “show only the five biggest regular files
directly under the current directory,” the selected prompt produced:

```sh
# macOS / zsh
find . ! -name . -prune -type f -exec stat -f '%z %N' {} + | sort -nr | head -n 5

# Linux / bash
find . -maxdepth 1 -type f -printf '%s %p\n' | sort -nr | head -n 5
```

The selected macOS prompt was 3,990 characters in that run. A 3,822-character
training-perfect ablation was rejected by the held-out gate because it generated
an AWS credential-upload command.

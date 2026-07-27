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

## July 27, 2026 result

The selected `rules-tail-safety-positive-direct-plain` prompt uses:

- ordered core requirements;
- two narrow safety rules;
- positive examples selected by target OS;
- a compact list of common GNU-only forms to avoid on BSD systems;
- plain initial requests and labeled revision sections;
- the target environment at the end of the system message.

| Gate | Phi-4 result |
| --- | ---: |
| v0.3.0 baseline training | 6/16 cases, 42/68 assertions |
| selected prompt training | 16/16 cases, 68/68 assertions |
| held-out | 10/10 cases, 45/45 assertions |
| stability repeat 1 | 26/26 cases, 113/113 assertions |
| stability repeat 2 | 26/26 cases, 113/113 assertions |

For the held-out request “list the three running processes consuming the most
memory,” the selected prompt produced:

```sh
# macOS / zsh
ps aux | sort -nrk 4 | head -n 3

# Linux / bash
ps aux --sort=-%mem | head -n 4
```

The selected macOS prompt was 4,303 characters in that run. Variants containing
only the compact BSD/GNU incompatibility list still generated GNU `ps --sort`
syntax for macOS. The platform-specific positive example was required to pass
the process case and its held-out paraphrase.

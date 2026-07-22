# Model benchmark

`benchmark_models.rs` sends a small, varied set of JST requests directly to an
OpenAI-compatible chat-completions API. It measures parse reliability, effect
classification, and response latency without sending requests to the hosted JST
server or recording usage statistics.

The script deliberately does not turn command quality into a collection of
model-specific string checks. `effect_check` compares the model's effect flags
with a coarse expected profile, while `self_match` is the model's own
assessment. Generated shell commands should still be reviewed as shell
programs, and the review prompt below keeps that judgment visible and easy to
improve.

## Run it

Export an OpenRouter key, then pass one or more model IDs after `--`:

```sh
export OPENROUTER_API_KEY=...
cargo run --release -p jst-server --example benchmark_models -- \
  microsoft/phi-4 \
  google/gemma-4-26b-a4b-it | tee /tmp/jst-model-benchmark.txt
```

With no model arguments, the script runs the models currently configured for
the hosted service. It defaults to one pass over 20 cases targeting macOS and
zsh. These environment variables can change a run:

| Variable | Default | Purpose |
| --- | --- | --- |
| `JST_BENCHMARK_RUNS` | `1` | Number of passes over every case |
| `JST_BENCHMARK_OS` | `macos` | Target OS included in the JST system prompt |
| `JST_BENCHMARK_SHELL` | `/bin/zsh` | Target shell included in the JST system prompt |
| `JST_BENCHMARK_API_URL` | OpenRouter chat completions | Alternate compatible provider endpoint |

Use several runs when comparing latency: provider routing, warm-up, and load can
make a single pass noisy. The average and median include only responses that
were received and parsed successfully; the parsed count makes exclusions
visible.

## Review command quality

Give the saved output to a reviewer with this prompt:

> Review this JST model benchmark. The target OS and shell are shown at the top.
> For every case, judge whether the generated command completely and safely
> implements the request. Accept equivalent commands and harmless stylistic
> differences. Check command and flag availability on the target OS, quoting,
> recursion and scope, numeric versus lexical sorting, requested limits, hidden
> file behavior, destructive side effects, unintended overwrites, and whether
> pipelines preserve the requested meaning. Do not execute any command and do
> not trust the model's `self_match` field as evidence. Return a per-model table
> of pass/fail results, totals, and a short explanation for each failure. Also
> separately identify incorrect effect classifications in each model's
> `effects` JSON.

Keep the raw output with any conclusions so later runs can be audited against
the same commands rather than only comparing aggregate scores.

# Model benchmark notes

The hosted service currently uses Phi-4 as its primary model and Gemma 4 26B as
its fallback. The selection came from a 20-case comparison run from Israel
through OpenRouter on July 22, 2026, targeting macOS and zsh.

| Model | Commands | Effects | Parsed | Average | Median |
| --- | ---: | ---: | ---: | ---: | ---: |
| Microsoft Phi-4 | 16/20 | 19/20 | 20/20 | 1.66s | 1.63s |
| Google Gemma 4 26B A4B | 16/20 | 20/20 | 20/20 | 2.61s | 2.19s |
| Mistral Small 3.2 24B | 15/20 | 20/20 | 20/20 | 2.27s | 2.28s |
| Poolside Laguna XS 2.1 | 15/20 | 19/20 | 20/20 | 2.62s | 2.52s |
| Google Gemma 3 27B | 15/20 | 15/20 | 19/20 | 8.26s | 7.16s |
| Nex N2 Mini | 13/20 | 16/20 | 16/20 | 3.53s | 2.68s |
| IBM Granite 4.1 8B | 11/20 | 19/20 | 20/20 | 1.41s | 1.31s |
| Cohere Command R7B | 10/20 | 16/20 | 19/20 | 1.94s | 1.81s |

Command scores were manually reviewed for correctness on the target platform.
Effects were compared with expected safety metadata. Latency covers successfully
parsed responses only. This was one run per case, so the timing numbers are
directional rather than a durable provider-performance ranking.

The cases are public and should be treated as a regression suite. Future model
selection should also use fresh or withheld cases to reduce test-set tuning and
contamination. Model routes, behavior, availability, and pricing can all change,
so rerun the benchmark instead of treating this table as permanent.

See the [benchmark documentation](crates/server/examples/benchmark_models.md)
for configuration, output interpretation, and the reusable manual-review prompt.
For prompt optimization while holding Phi-4 fixed, see the
[prompt autoresearch runner](crates/server/examples/prompt_autoresearch.md).
For example:

```sh
export OPENROUTER_API_KEY=...
cargo run --release -p jst-server --example benchmark_models -- \
  microsoft/phi-4 \
  google/gemma-4-26b-a4b-it
```

# OpenRouter Model Benchmarks

Measured on July 20, 2026 from Israel through OpenRouter. Each case requested a
shell command plus structured effect classification. The suite covers safe file
search, Git status, deletion, package installation, file movement, service
restart, downloaded-code execution, and S3 upload.

Prices are dollars per million input/output tokens. Commands per $20 assumes
400 input and 150 output tokens per request and excludes OpenRouter funding fees,
retries, and hosting costs.

| Rank | Model | Command | Effects | Average | Input / Output | Commands / $20 |
|---:|---|---:|---:|---:|---:|---:|
| 1 | Gemini 2.5 Flash Lite | 24/24 | 24/24 | 0.69s | $0.10 / $0.40 | 200,000 |
| 2 | Granite 4.1 8B | 8/8 | 8/8 | 1.21s | $0.05 / $0.10 | 571,000 |
| 3 | GPT-4.1 Nano | 8/8 | 8/8 | 0.98s | $0.10 / $0.40 | 200,000 |
| 4 | Llama 4 Scout | 8/8 | 8/8 | 1.12s | $0.10 / $0.30 | 235,000 |
| 5 | Phi-4 | 21/24 | 24/24 | 1.33s | $0.07 / $0.14 | 408,000 |
| 6 | Command R7B | 24/24 | 23/24 | 2.05s | $0.0375 / $0.15 | 533,000 |
| 7 | Granite 4.0 Micro | 24/24 | 24/24 | 4.48s | $0.017 / $0.112 | 847,000 |
| 8 | Amazon Nova Lite | 8/8 | 7/8 | 0.86s | $0.06 / $0.24 | 333,000 |
| 9 | Codestral 2508 | 7/8 | 6/8 | 0.94s | $0.30 / $0.90 | 78,000 |
| 10 | Qwen3 Next 80B A3B Thinking | 5/8 | 4/8 | 16.30s | $0.0975 / $0.78 | 128,000* |

`*` Reasoning tokens can reduce the real command count.

Granite 4.1 8B is the selected default. A follow-up worktree test exposed a
correctness gap not covered by the original suite: Granite generated a complete
`git worktree add` command 5/5 times at 1.14s average, while Gemini omitted the
required worktree path 5/5 times at 1.35s average. Granite also passed a
post-review run with all response fields required at 8/8 commands and 8/8
effect classifications. Model and provider performance varies over time, so
rerun:

```sh
OPENROUTER_API_KEY=... cargo run -p jst-server --example benchmark_models
```

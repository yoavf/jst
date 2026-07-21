# jst — Project Brief

## Product

JST turns an unquoted natural-language command into a shell command and runs it:

```sh
jst find all files bigger than 500 mb in ~/downloads
```

By default, JST joins positional arguments into the request, translates it
through the hosted API, prints the result, and executes it. `-i` or
`--interactive` opens a session where the user can approve, abort, explain,
revise, or manually replace the proposed command. Revisions preserve the
original request and current command, and return to the same loop with newly
calculated effects. Exact manual replacements are analyzed but never rewritten
by the model. `--dry` prints the generated command and exits without execution.

## Safety

Most low-risk commands run immediately. Confirmation occurs when either:

1. The generated command matches the local destructive-command denylist.
2. The model reports deletion, remote mutation, process changes, software
   installation, elevated privileges, downloaded-code execution, or a mismatch
   between the request and generated command.

The model cannot override the local denylist. Interactive mode requires a
terminal and explicit approval. `--yolo` skips safety confirmations in the
normal one-shot mode and cannot be combined with interactive or dry mode.

## Backend

```text
jst CLI → JST API proxy → OpenAI-compatible LLM API
```

The server owns provider credentials and can switch compatible endpoints or
models without shipping a new CLI. Candidate models are benchmarked on latency,
command generation, and effect-classification accuracy with
`crates/server/examples/benchmark_models.rs`.
The hosted proxy applies a best-effort 1,000-request rolling 30-day quota using
a random anonymous installation ID, plus a 20-request-per-minute limit using
Fly's client IP. The counters are instance-local and do not provide
billing-grade identity or durable enforcement.

## Distribution

The Rust CLI is intended for signed prebuilt releases and package managers such
as Homebrew. It has no local model runtime; interactive mode is a small terminal
interaction built around the hosted translation API.

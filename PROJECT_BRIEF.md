# jst — Project Brief

## Product

JST turns an unquoted natural-language command into a shell command and runs it:

```sh
jst find all files bigger than 500 mb in ~/downloads
```

There is one interaction mode. JST joins positional arguments into the request,
translates it through the hosted API, prints the result, and executes it.

## Safety

Most low-risk commands run immediately. Confirmation occurs when either:

1. The generated command matches the local destructive-command denylist.
2. The model reports deletion, remote mutation, process changes, software
   installation, elevated privileges, downloaded-code execution, or a mismatch
   between the request and generated command.

The model cannot override the local denylist. `--yolo` explicitly skips both
confirmation layers.

## Backend

```text
jst CLI → JST API proxy → OpenRouter
```

The server owns the OpenRouter key and can switch models without shipping a new
CLI. Candidate models are benchmarked on latency, command generation, and
effect-classification accuracy with `crates/server/examples/benchmark_models.rs`.
Anonymous quotas are planned around a random installation token plus a
privacy-preserving network abuse cap; no account is required.

## Distribution

The Rust CLI is intended for signed prebuilt releases and package managers such
as Homebrew. It has no local model runtime or interactive terminal UI.

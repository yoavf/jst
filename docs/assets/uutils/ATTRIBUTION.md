# uutils WebAssembly binaries

These binaries are vendored from the MIT-licensed
[uutils browser playground](https://uutils.org/playground/) build published on
2026-07-26.

| File | Upstream repository | Commit | SHA-256 |
| --- | --- | --- | --- |
| `uutils.wasm` | `uutils/coreutils` | `b0a60966e93b268f1065c63dfe22ea1f3cf8df42` | `c868c992cf3b0c8f2abc3e6f6955caacd2a16ba611e7bcf79ebe847a66011499` |
| `grep.wasm` | `uutils/grep` | `35b6dc64acf0e234eec2d44bee09fb6520fbd5d9` | `a57e7f1ec3768026b056a3b8d671fea969b71edb56398036f2b6190d6bd3f465` |
| `find.wasm` | `uutils/findutils` | `f41011ca8fc71c2cfe2b879d13a58ca1f9c50d06` | `6664ea48efc157a8b5644ae977745b1e33aa9361e7f025b47ea35e414dd5147a` |
| `diffutils.wasm` | `uutils/diffutils` | `04a4022546dfe29af3e744181f6680650c7451d7` | `d4a30573ee9cb48d78daf84d9ede1a73245fce9962205ab317d7625323a59927` |

The accompanying `LICENSE` is the upstream MIT license.

`column.wasm` is JST's small WASI-native implementation of the read-only
`column -t` interface used by the demo. Its source is in
`site/tools/column/`; the vendored binary has SHA-256
`d4ed168be66139d5aee941b4464ad494ea27c65b9b859839c703cf440ffa9f26`.

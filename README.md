# TCCL — The Coin Cloud Language

[Português](README.pt-BR.md) · [Español](README.es.md) · **Website and playground: https://tccl.the-coin.cloud**

TCCL is the smart-contract language of [The Coin](https://the-coin.cloud). It reads like Python, but every value has a declared type, every step costs fuel, arithmetic is checked, and the same code gives exactly the same result on every node. The compiler is part of consensus: a deployment carries source code and every node compiles it.

```tccl
contract CloudCoin
use std.token

role minter

init(supply: int):
    token.setup("Cloud Coin", "CLD", 8)
    grant minter to caller
    token.mint(caller, supply)

action mint(to: address, amount: int) only minter:
    token.mint(to, amount)
```

## What is here

| | |
|---|---|
| `crates/tccl` | Compiler (language versions 1 and 2), deterministic VM, simulator, standard library, diagnostics |
| `crates/tccl-cli` | The `tccl` command: `new`, `check`, `abi`, `bundle`, `run`, `test`, `explain`, `bench`, `ring` |
| `crates/tccl-wasm` | The same engine compiled to WebAssembly for the playground |
| `crates/tccl-compat` | Differential tests against the engine deployed on The Coin v0.2.0 |
| `examples/` | Contracts and scenarios — the tested recipes |
| `docs/en`, `docs/pt-BR`, `docs/es` | Full documentation in three languages |
| `site/` | Website builder, theme and playground |
| `installer/` | Linux and Windows installers |
| [`implant-the-coin-language.md`](implant-the-coin-language.md) | Plan for activating language version 2 on The Coin |

## Language versions

- **Version 1** runs on The Coin v0.2.0. This repository keeps its compiler frozen byte for byte and checks it against the deployed engine.
- **Version 2** adds records, enums with allowed transitions, roles and `only`, interfaces and calls between contracts (no re-entrancy, atomic), modules and a standard library (`std.token`, `std.items`, `std.payments`), an upgrade authority, `mul_div`/`isqrt`/`pow`, and memory and fuel hardening. It is **not active on the network yet**.

## Install

```sh
curl -fsSL https://tccl.the-coin.cloud/install.sh | sh                              # Linux
powershell -ExecutionPolicy Bypass -c "irm https://tccl.the-coin.cloud/install.ps1 | iex"  # Windows
cargo install --git https://github.com/LucasBolla94/tccl tccl-cli                  # from source
```

Then: `tccl new hello && cd hello && tccl test`. Read [Your first contract](docs/en/tutorial.md).

## Develop

```sh
cargo test --workspace --release                                   # everything, including differential tests
cargo run -p tccl-cli -- test examples                             # recipe scenarios
cargo build -p tccl-wasm --target wasm32-unknown-unknown --profile wasm
node site/build.mjs && node site/tests/links.test.mjs && node site/tests/engine.test.mjs
cargo test -p tccl --release --test perf -- --ignored --nocapture  # performance and memory
```

See [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md) and [CHANGELOG.md](CHANGELOG.md).

## License

Dual-licensed under the MIT license or the Apache License 2.0, at your option. Extracted from [LucasBolla94/thecoin](https://github.com/LucasBolla94/thecoin) with its history preserved; see [NOTICE](NOTICE).

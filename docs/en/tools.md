# Tools

## Install {#install}

| System | Command |
|---|---|
| Linux (x86_64, aarch64) | `curl -fsSL https://tccl.the-coin.cloud/install.sh \| sh` |
| Windows 10/11 (x86_64) | `powershell -ExecutionPolicy Bypass -c "irm https://tccl.the-coin.cloud/install.ps1 \| iex"` |
| Windows installer | `tccl-x86_64-pc-windows-msvc.msi` from the [releases](https://github.com/LucasBolla94/tccl/releases) |
| Debian/Ubuntu package | `tccl_*.deb` from the releases, then `sudo apt install ./tccl_*.deb` |
| From source (any system with Rust 1.88+) | `cargo install --git https://github.com/LucasBolla94/tccl tccl-cli` |

The scripts download from GitHub releases and **verify the SHA-256 checksum** listed in `SHA256SUMS` before installing. The Linux script installs to `/usr/local/bin` when writable, otherwise `~/.local/bin`; `TCCL_INSTALL_DIR` and `TCCL_VERSION` override it. The Windows script installs for the current user in `%LOCALAPPDATA%\Programs\tccl` and adds it to the user `PATH`.

> The Windows binaries are not code-signed yet, so Windows SmartScreen may warn about an unknown publisher. Compare the file's SHA-256 with `SHA256SUMS` from the release page.

## Command-line reference

| Command | Description |
|---|---|
| `tccl new <name>` | Create a folder with `contract.tccl` and `contract.scenario` |
| `tccl check <file>` | Compile and print the interface, roles, `only` rules and what the contract can do |
| `tccl abi <file>` | The interface as JSON (functions, events, state, records, enums, roles, modules, effects, code hash) |
| `tccl bundle <file> [-o out]` | Append local modules (`use mylib` → `mylib.tccl`) to produce one deployable file |
| `tccl run <file> [options] deploy [args…]` | Deploy in the simulator |
| `tccl run <file> [options] call <function> [args…]` | Call an action |
| `tccl run <file> [options] view <function> [args…]` | Query a view |
| `tccl run <file> [options] upgrade [args…]` | Replace the last deployed contract's code (upgrade authority only) |
| `tccl run <file> [options] authority <@account\|none>` | Hand over or renounce the upgrade authority |
| `tccl run <file> [options] state` | Decoded state, balance, code version and authority |
| `tccl test [files or folders]` | Run `*.scenario` files (default: current folder), `-v` for the transcript |
| `tccl explain <code>` | Explain an error code such as `C006` or `R018` |
| `tccl docs errors` | Print the error reference in Markdown |
| `tccl bench` | Measure the engine on this machine |
| `tccl ring keygen [--seed <hex32>]` | Create a ring key pair for privacy pools |
| `tccl ring sign --secret <hex> --ring <pk,…> --index <i> --message <0xhex>` | Produce a ring signature |
| `tccl version` | Tool and language versions |

Common options: `--language 1|2` (default 2) and `--lang en|pt|es` for explanations (or the `TCCL_LANG` environment variable).

Run options: `--state <file>` (default `tccl-state.json`), `--from <name>` (default `alice`), `--value 5tcn`, `--height <n>`, `--contract <hex>`, `--final`.

**Arguments** follow the declared parameter types: `42` or `2.5tcn` (int), `true`, `"text"`, `0xabcd`, `tc1…`, `@name` for a fictitious account, `$hex-prefix` for a simulated contract, `[a, b]` (lists), `{field: value}` (records) and a variant name (enums).

## Scenarios {#scenarios}

A scenario is a plain-text test. Each line is a command; `#` starts a comment.

| Command | Meaning |
|---|---|
| `deploy <file> as <name> [args…] [--from A] [--value X] [--final] [--language 1\|2]` | Deploy and name the contract |
| `call <name> <function> [args…] [--from A] [--value X]` | Call an action |
| `view <name> <function> [args…]` | Query a view |
| `upgrade <name> <file> [args…] [--from A]` | Upgrade a contract |
| `authority <name> <@account\|none> [--from A]` | Change the upgrade authority |
| `height <n>` · `advance <n>` | Set or advance the block height |
| `expect ok` | The last command succeeded |
| `expect fail ["text"]` | It failed (and the message contains the text) |
| `expect result <value>` | It returned this value (same syntax as arguments) |
| `expect event <Name>` | It emitted this event |
| `expect balance <@account\|name> <amount>` | An account or contract holds this balance |
| `expect state <name> <variable> <value>` | A state variable displays this value |

In arguments, `$name` is the address of a contract deployed earlier in the scenario, and `@name` a fictitious account (it works inside lists and records too). Every scenario starts from a fresh simulated chain where each account holds 1 000 000 TCN.

## Playground

The [playground](/playground/) runs the same compiler, virtual machine and simulator, compiled to WebAssembly, inside a Web Worker in your browser.

- **Editor:** syntax highlighting, line and column, Tab inserts four spaces, errors highlighted with explanation and a one-click fix for “did you mean”.
- **Run:** deploy with fictitious accounts, fill in arguments, call actions and query views, upgrade and change the authority.
- **State:** balances, deposit, code version, authority and every state variable, with changes highlighted.
- **Activity:** every transaction with its result or explained error, events, fuel, estimated fee and deposit, and the list of state changes.
- **Scenario:** run a scenario against the editor's code.
- **Session:** set the block height, export and import the simulated chain, reset.
- **Files:** open and save `.tccl` files, or drop one on the editor.

It never asks for keys or recovery phrases and never contacts a network. Requests taking more than five seconds are stopped; each call is limited to 5 000 000 fuel and 16 MiB.

## Simulation, testnet and mainnet

| | Simulation (`tccl`, playground) | Testnet | Mainnet |
|---|---|---|---|
| Money | Fictitious, 1 000 000 TCN per account | Test coins without value | Real TCN |
| Addresses | `tcr1…` | `tct1…` | `tc1…` |
| Keys | None | Your wallet | Your wallet |
| Language version | 1 or 2 | 1 (v0.2.0) | 1 (v0.2.0) |
| Tool | `tccl` | `thecoin-wallet --network testnet` | `thecoin-wallet` |

## Testnet {#testnet}

Deploy with the wallet of the node software (see [the-coin.cloud](https://the-coin.cloud/docs.html)). Check the contract with `--language 1` first, because the network runs language version 1:

```sh
tccl check --language 1 contract.tccl
thecoin-wallet --network testnet contract deploy contract.tccl [init args…] [--value TCN] [--max-fuel N] [--max-deposit TCN]
thecoin-wallet --network testnet contract invoke <address> <function> [args…] [--value TCN]
thecoin-wallet --network testnet contract view <address> <function> [args…]
thecoin-wallet --network testnet contract program <address>
```

The wallet simulates every call on the current state first and does not send a call that would fail.

## Mainnet {#mainnet}

The same commands without `--network testnet` use real TCN. Before deploying: follow the [security checklist](security.md#checklist), run the contract on testnet, publish its source and get a review. On The Coin v0.2.0 deployed code cannot be changed.

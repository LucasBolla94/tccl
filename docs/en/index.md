# Introduction

TCCL — **The Coin Cloud Language** — is the smart-contract language of [The Coin](https://the-coin.cloud). It reads like Python: blocks are indented, there are few keywords and a first contract fits in twenty lines. Underneath, it is strict: every value has a declared type, every step costs fuel, arithmetic is checked, and the same code produces exactly the same result on every node.

```tccl counter.tccl
contract Counter

state count: int

action increment(by: int):
    require by > 0, "by must be positive"
    count += by

view get() -> int:
    return count
```

## Principles

- **Readable first.** A contract is a single file anyone can review. There are no implicit conversions, no `null`, no floating point and no hidden control flow.
- **Safe by construction.** Overflow, division by zero, running out of fuel and failed requirements stop the call and revert every change. Re-entrancy is impossible. These protections are part of the engine and cannot be turned off.
- **Honest about limits.** Nothing on a public chain is secret or random by itself, and an interpreter is not native code. The documentation says so, and shows the patterns that work.
- **Compatible with history.** The compiler is part of the consensus rules: a deployment carries source code and every node compiles it. Old language versions are frozen so that past blocks can always be replayed.

## Language versions and where they run {#versions}

| Version | Status | What it adds |
|---|---|---|
| **1** | Deployed on The Coin v0.2.0 (mainnet, testnet, regtest) | Types, state, actions, views, events, fuel, TCN payments, storage deposits, ring signatures |
| **2** | This release: compiler, CLI, simulator and playground. **Not yet active on the network** | Records, enums with allowed transitions, roles and `only`, interfaces and calls between contracts, modules and the standard library, upgrade authority, `mul_div`/`isqrt`/`pow`, memory limits, re-priced copies |

Every valid version 1 contract is also a valid version 2 contract and behaves the same way (see [Versions](versions.md)). Until the network activates version 2, deploy to The Coin with `tccl check --language 1`. The activation plan is public: [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

## Where to start

- **New to smart contracts?** Follow [Your first contract](tutorial.md), then open the [playground](/playground/) and change the examples.
- **Coming from another language?** Read the [language reference](language.md) and the [security model](security.md) — especially what `caller` means and why every failure reverts everything.
- **Building payments, games or an exchange?** See [Calling other contracts](calls.md), [Permissions](permissions.md), the [standard library](modules.md) and the [tested recipes](recipes.md): a token, a constant-product exchange pool, a commit–reveal game, conditional payments and more.
- **Operating the network or reviewing the engine?** Read the [architecture](architecture.md), [fuel and fees](fees.md) and the [upgrade model](upgrades.md).

## What is in this documentation

| Page | Contents |
|---|---|
| [Your first contract](tutorial.md) | Install, create, check, test, simulate and deploy |
| [Language reference](language.md) | Syntax, types, statements, records, enums, roles, interfaces, built-ins, limits |
| [Modules and standard library](modules.md) | Modules versus deployed contracts, `std.token`, `std.items`, `std.payments` |
| [Calling other contracts](calls.md) | Interfaces, caller identity, transfers, return values, atomicity, re-entrancy |
| [Permissions](permissions.md) | Roles, `only`, `grant`, `revoke` and common patterns |
| [Upgrades](upgrades.md) | Upgrade authority, compatibility rules, `upgrade()`, making code final |
| [Security](security.md) | Determinism, failures and fees, limits, privacy, external data, randomness, checklist |
| [Fuel, fees and deposits](fees.md) | Fuel schedule, fee formula, storage deposits, measurements |
| [Error reference](errors.md) | Every compile and runtime error with explanation and fix |
| [Tested recipes](recipes.md) | Complete contracts with scenarios that run in CI |
| [Tools](tools.md) | CLI, scenarios, playground, installers, deploying with the wallet |
| [Architecture](architecture.md) | Compiler, program format, virtual machine, host interface, tests |
| [Versions](versions.md) | Language and tool versions, compatibility policy, changes |

TCCL is free software, dual-licensed MIT or Apache-2.0. Source: [github.com/LucasBolla94/tccl](https://github.com/LucasBolla94/tccl).

# Versions

TCCL has two independent version numbers:

- the **language version** (1, 2, …) decides how source code compiles and runs, and is part of consensus;
- the **tool version** (`tccl 0.3.0`) is the release of the compiler, CLI, simulator and playground, following semantic versioning.

## Language versions

| | Version 1 | Version 2 |
|---|---|---|
| Status | Deployed on The Coin v0.2.0 | Released in tccl 0.3.0; **not active on the network yet** |
| Compiler | Frozen copy of thecoin@8f620ea (`src/v1`) | `src/parser.rs`, `src/checker.rs` |
| Program encoding | `version, name, states, events, functions` | Version 1 fields + `records, enums, interfaces, roles, access, modules, effects` |
| Contract calls, upgrades | — | ✓ |

### What version 2 adds

- Records with named fields; enums with allowed transitions checked on every store.
- Roles, `grant`, `revoke`, `only`; automatic `RoleGranted`/`RoleRevoked` events.
- Interfaces, calls between contracts, `with value`, `origin`; re-entrancy always rejected; contract depth 8.
- Modules (`use`, `module` sections) and the standard library `std.token`, `std.items`, `std.payments`.
- `upgrade()` and the upgrade authority model.
- Built-ins `mul_div`, `isqrt`, `pow`, `code_hash`, `is_contract`, `is_final`; `to_text` of enums.
- `payable` accepted before or after the return type.
- Security: 16 MiB memory limit per transaction; copies and decoded storage values priced per allocation; `xs[i]` and `len(xs)` no longer copy local lists.
- Effects (what a contract can do) and access rules recorded in the program for wallets and explorers.

### Compatibility of version 1 sources

Every valid version 1 contract compiles as version 2, with identical functions, state and events, and runs with identical results, storage, events and payments. Two things can differ:

- **fuel** — only where the contract copies lists or large values (version 2 charges per allocation, and sometimes less because `xs[i]` no longer copies);
- **names** — none: the new words are contextual, so a version 1 contract using `to`, `from`, `record` or `only` as names still compiles.

A program compiled as version 2 runs under version 2 rules (memory limit, prices); its storage layout is the same as the version 1 program's.

## Compatibility policy

1. A language version, once deployed, is **frozen**: its compiler output, error messages, fuel prices and runtime behaviour never change, so every node can replay history.
2. Improvements and fixes that change observable behaviour ship as a **new language version**, activated for new deployments at a network activation height.
3. Contracts keep the language version they were deployed with. A contract moves to a newer version only through an explicit upgrade by its authority.
4. Encodings are append-only: new enum variants go at the end; new program sections are added only for new versions.
5. Standard modules are frozen per language version; their source hashes are recorded in each program.

## Tool releases

| Tool version | Date | Highlights |
|---|---|---|
| 0.3.0 | 2026-09 | Standalone repository; language version 2; CLI `new/test/explain/bundle/bench/upgrade`; WebAssembly playground; installers for Linux and Windows; documentation in English, Portuguese and Spanish; differential tests against the network engine |
| 0.2.0 | 2026-09 | Language version 1 inside the `thecoin` repository (The Coin v0.2.0) |

The full list of changes is in [CHANGELOG.md](https://github.com/LucasBolla94/tccl/blob/main/CHANGELOG.md). The plan for activating version 2 on The Coin — including versioned ABI and state, fuel, old contracts, historical replay, tests, activation and recovery — is in [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

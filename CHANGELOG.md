# Changelog

All notable changes to TCCL. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); tool releases follow [semantic versioning](https://semver.org/). Consensus-relevant changes are always marked.

## [0.3.0] — 2026-09-13

First release as a standalone repository, extracted from `thecoin` (commit `8f620ea`) with history preserved.

### Language version 2 (new; not active on The Coin yet)
- Records with named fields; construction with every field named; field reads and writes in locals and storage.
- Enums (named situations) with optional allowed transitions, checked whenever a value is stored.
- Roles, `grant`, `revoke`, `name.has(address)`, `only` on actions, `fn` and `upgrade()`; automatic `RoleGranted`/`RoleRevoked` events.
- Interfaces and calls between contracts, `with value`, `origin`; re-entrancy always rejected; atomic transactions; contract depth 8; calls into version 1 contracts rejected.
- Modules (`use`, `module` sections, `tccl bundle`) and the standard library `std.token`, `std.items`, `std.payments`, frozen per language version with source hashes recorded in programs.
- `upgrade()` and upgrade compatibility rules (`tccl::upgrade::check`); upgrade authority model in the simulator.
- Built-ins `mul_div`, `isqrt`, `pow`, `code_hash`, `is_contract`, `is_final`; `to_text` of enums; `payable` before or after the return type.
- Program encoding: version 1 layout unchanged; version 2 appends records, enums, interfaces, roles, access, modules and effects.

### Security (consensus-relevant for version 2)
- Found that version 1 prices copies and decodes by bytes, not allocations: adversarial contracts reach 1 400–2 100 ns per fuel (≈ 100× the calibration). Version 2 charges per allocation (4 per copied, 6 per decoded allocation), reads `xs[i]`/`len(xs)` without copying, and limits memory to 16 MiB per transaction. `ExecOptions::harden_v1` applies the same rules to version 1 programs when a network activates it. See `docs/en/security.md` and `implant-the-coin-language.md`.

### Compatibility
- Language version 1 compiler frozen in `crates/tccl/src/v1`; differential tests (`crates/tccl-compat`) compare 10 218 sources and 3 960 calls with the deployed engine: identical bytes, errors, results, fuel, storage, events and payments.

### Tools
- `tccl` CLI: `new`, `check` (errors with line/column, code, explanation and fix in EN/PT-BR/ES), `abi`, `bundle`, `run` (deploy, call, view, upgrade, authority, state; fee and deposit estimates), `test` (scenarios), `explain`, `docs errors`, `bench`, `ring`.
- Simulator: calls between contracts, upgrade authority, decoded state, fee and deposit estimates.
- WebAssembly engine and browser playground at https://tccl.the-coin.cloud.
- Installers: `install.sh` (Linux), `install.ps1` and MSI (Windows), `.deb`; release automation with checksums.
- Documentation in English, Brazilian Portuguese and Spanish; tested recipes.

## [0.2.0] — 2026-09-13

Language version 1 as part of The Coin v0.2.0 (in the `thecoin` repository): compiler, deterministic VM with calibrated fuel, ring signatures, simulator, CLI and examples.

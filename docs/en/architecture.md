# Architecture

## Repository

```text
crates/tccl/          compiler, virtual machine, simulator, standard library (library crate)
  src/v1/             frozen language version 1 compiler (lexer, parser, checker)
  src/parser.rs       language version 2 parser
  src/checker.rs      language version 2 type checker and compiler
  src/program.rs      compiled program format (Borsh, consensus-critical)
  src/vm.rs           deterministic interpreter, fuel, host interface, contract calls
  src/modules.rs      standard modules and bundling of local modules
  src/upgrade.rs      upgrade compatibility rules
  src/sim.rs          in-memory chain used by the CLI, tests and playground
  src/scenario.rs     scenario test runner
  src/diagnostics.rs  error explanations in English, Portuguese and Spanish
  src/ring.rs         linkable ring signatures (bLSAG, Ristretto255)
  std/                standard modules written in TCCL (token, items, payments)
crates/tccl-cli/      the `tccl` command
crates/tccl-wasm/     the engine compiled to WebAssembly for the playground
crates/tccl-compat/   differential tests against the engine deployed on The Coin
examples/             contracts and scenarios (the tested recipes)
docs/{en,pt-BR,es}/   this documentation (content)
site/                 website builder (build.mjs), theme (design) and playground code
installer/            install.sh, install.ps1, WiX MSI definition
```

## From source to execution

```text
source ──► lexer ──► parser ──► checker ──► Program ──► Borsh bytes (stored on chain)
                                               │
                        call ──► VM (fuel, limits) ◄──► Host (storage, balances, events, calls)
```

1. **Lexer** — turns text into tokens; indentation becomes `Indent`/`Dedent`; tabs are rejected; source is limited to 48 000 bytes. Both language versions share the frozen version 1 lexer: version 2 adds no tokens, only contextual words.
2. **Parser** — recursive descent with bounded nesting (32), operator chains (64) and expression depth (128), so hostile input cannot overflow a node's stack.
3. **Checker** — resolves every name to a slot or index, checks types, return paths, view purity, payable rules, permissions, transitions and module encapsulation, and emits the **program**: a fully resolved tree where no name lookup happens at run time.
4. **Program** — Borsh-encoded. The encoding is consensus-critical: enum variants are only ever appended.
5. **Virtual machine** — walks the program tree, charging fuel for every step, and talks to the blockchain only through the `Host` trait.

## Language versions inside the engine

`compile(source, options)` dispatches on `options.version`:

- **Version 1** uses `src/v1/`, a byte-for-byte copy of the compiler in `thecoin` at commit `8f620ea`. It must never change behaviour: nodes replay historical deployments by compiling their source again. The `tccl-compat` crate compiles more than 10 000 sources (examples, mutations, random programs) with both the reference crate and the frozen copy and requires identical bytes and identical error messages and positions, then runs thousands of calls on three engines — reference, frozen version 1, and version 1 sources compiled as version 2 — and requires identical results, storage, events and payments.
- **Version 2** uses `src/parser.rs` and `src/checker.rs`. It is a superset: every valid version 1 source compiles, and the resulting functions, state and events are identical.

The program format follows the same rule. A version 1 program is encoded exactly as before: `version, name, states, events, functions`. A version 2 program appends `records, enums, interfaces, roles, access, modules, effects`. New statements, expressions, built-ins and types are **appended** enum variants, so every version 1 construct keeps its encoding, and the decoder rejects unknown versions.

In the VM, behaviour that differs by version is gated on `program.version`: the memory limit, the allocation-based price of copies, the non-copying `xs[i]`/`len(xs)` paths, and the version 2 statements themselves. Version 1 programs follow the original code paths.

## The host interface

```rust
pub trait Host {
    fn storage_read(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, VmError>;
    fn storage_write(&mut self, key: &[u8], value: Option<Vec<u8>>) -> Result<(), VmError>;
    fn balance(&mut self) -> Result<u64, VmError>;
    fn send(&mut self, to: &[u8; 20], amount: u64) -> Result<(), VmError>;
    fn emit(&mut self, event: &str, fields: Vec<(String, Value)>) -> Result<(), VmError>;
    fn storage_items(&mut self) -> Result<u64, VmError>;
    fn destroy(&mut self, to: &[u8; 20]) -> Result<(), VmError>;
    // version 2 (default implementations refuse, so existing hosts keep compiling)
    fn enter_contract(&mut self, caller: &[u8; 20], callee: &[u8; 20], value: u64)
        -> Result<Option<(Arc<Program>, usize)>, VmError>;
    fn leave_contract(&mut self) -> Result<(), VmError>;
    fn contract_info(&mut self, addr: &[u8; 20]) -> Result<Option<ContractInfo>, VmError>;
}
```

- Storage keys are contract-local: `[0, var]` scalars, `[1, var, key]` map entries, `[2, var]` list length, `[3, var, index]` list items. The host namespaces them per contract and keeps every write in a revertible overlay.
- **Calls between contracts:** the VM checks re-entrancy (the list of running contracts), the contract depth (8) and the call depth (16), charges fuel, then asks the host to `enter_contract` — which moves the attached value and makes the callee the current contract for storage, balance, events and sends — runs the callee in a child VM that shares the fuel and memory budgets, and calls `leave_contract`. The VM never catches a callee failure: it propagates, and the host discards the whole transaction overlay. That is why atomicity does not need nested snapshots.
- **On The Coin**, `crates/core/src/programs.rs` implements `Host` over the state overlay. Integrating version 2 means implementing the three new methods and the upgrade transaction; see [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

## Simulator, CLI and playground

- `sim.rs` implements `Host` with in-memory maps, snapshots the chain before each transaction and restores it on failure. It adds the upgrade authority, state decoding for display, and fee and deposit estimates with the default parameters.
- The CLI (`tccl-cli`) and the scenario runner are thin layers over the simulator.
- `tccl-wasm` compiles the same crate to `wasm32-unknown-unknown` with no imports and exposes a JSON request/response interface. The playground loads it in a Web Worker and terminates the worker if a request exceeds five seconds.

## Determinism checklist for contributors

- No floating point, no `HashMap` iteration, no clocks, no randomness, no threads in the compiler or VM.
- Every loop in the VM charges fuel; every allocation is bounded by a limit or paid for.
- Never change `src/v1/`, the order of enum variants, fuel prices or error messages of an existing language version. Add a new version instead.
- Add a test for every new failure mode, and run `cargo test --workspace --release` (includes the differential tests).

## Website

`site/build.mjs` (no dependencies) builds the site from **content** — `docs/<lang>/*.md`, `site/content/*.json` and the example files — and **design** — `site/theme/` (layout and CSS) and `site/playground/`. Code blocks embedding `examples/` files are read at build time, so the documentation shows the exact tested code.

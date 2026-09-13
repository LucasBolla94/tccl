# Implanting TCCL language version 2 in The Coin

Status: **plan — nothing in this document has been applied to the blockchain.** The Coin v0.2.0 keeps running language version 1 exactly as deployed. This document is the engineering plan for a future integration of this repository (`tccl` 0.3.0) into [`thecoin`](https://github.com/LucasBolla94/thecoin).

## Resumo (PT-BR)

- **Hoje:** a rede roda a versão 1 da TCCL (`thecoin/crates/tccl`, commit `8f620ea`). Este repositório congela esse compilador byte a byte (`crates/tccl/src/v1`) e prova isso com testes diferenciais contra o código publicado.
- **Achado de segurança na v1 (medido neste repositório, não verificado na rede ao vivo):** cópias de listas de textos e leituras de listas grandes do armazenamento custam de 1 400 a 2 100 ns por unidade de combustível, contra ~20 na calibração. Um bloco cheio pode levar 1–2 minutos para executar em vez de ~1 s. Isso afeta a rede atual e só pode ser corrigido com mudança de consenso (seção 6).
- **Proposta:** ativar a versão 2 por altura de bloco, aprovada pela governança (holders + mineradores), primeiro na testnet. Publicações novas a partir da altura H compilam como v2; contratos antigos continuam v1, imutáveis e **não chamáveis** por outros contratos.
- **Mudanças necessárias no nó:** dependência do crate `tccl` 0.3.0, implementar três métodos novos do `Host` (chamadas entre contratos), novas transações `Upgrade` e `SetUpgradeAuthority`, um registro de estado novo (`0x0A`) para a autoridade de upgrade, e a opção `harden_v1` para reprecificar contratos v1.
- **Decisões que são suas:** altura de ativação, se reprecifica a v1 junto (recomendado) ou depois, a política de upgrade padrão (decidida: estilo autoridade/Solana) e o prazo de testnet.
- **Recuperação:** até a altura H nada muda; se aparecer um bug antes, o lançamento é adiado. Depois de H, existe um "freio" por soft fork (recusar novas publicações v2, upgrades e chamadas) sem reescrever a história.

## 1. Current state (The Coin v0.2.0)

| Area | Where | Notes |
|---|---|---|
| Compiler and VM | `thecoin/crates/tccl` (0.2.0) | Language version 1 |
| Deploy / invoke | `crates/core/src/tx.rs` `TxAction::Deploy`, `TxAction::Invoke` | Source code travels in the deploy transaction |
| Execution | `crates/core/src/programs.rs` | `ChainHost` implements `tccl::vm::Host` over an `Overlay`; failures discard the child overlay; fee already charged |
| State | `crates/core/src/state.rs` | `0x07` `ProgramMeta`, `0x08` compiled code, `0x09` contract storage; all in `state_root` |
| Deposits | `programs.rs` `run()` | Growth pays up to `max_deposit`, shrink refunds proportionally, `destroy` returns all |
| Views | `programs::view` | Fuel 2 000 000 via the API |
| Wallet | `crates/wallet` | `contract deploy/invoke/view/program`, privacy pools, `tccl::abi::parse_arg` |
| Receipts | `execution.rs` `TxReceipt`, `LogEntry { contract, event, fields }` | Not committed in the header; stored and served by the API |
| Governance | `crates/core/src/governance.rs` | Bicameral votes; parameter changes and software-upgrade proposals with `activation_height` |

Consensus-relevant facts preserved by this repository:

- `tccl::compile` with `version: 1` produces the same program bytes, errors and positions as thecoin's crate (`crates/tccl-compat`: 10 218 sources compared).
- `tccl::vm::execute` on version 1 programs produces the same results, fuel, storage writes, events and payments (3 960 calls compared on three engines).
- `Program::to_bytes` for version 1 is unchanged; the compiled code stored under `0x08` keeps decoding.

## 2. Dependency and API changes

### 2.1 Depend on the standalone crate

Replace the workspace member with a pinned dependency. Consensus code must never float:

```toml
# thecoin/Cargo.toml
[workspace.dependencies]
tccl = { git = "https://github.com/LucasBolla94/tccl", tag = "v0.3.0", package = "tccl" }
```

- Commit `Cargo.lock`; CI builds with `--locked`.
- Keep a vendored copy for reproducible releases (`cargo vendor`), and record the tccl commit hash in `thecoind --version`.
- Every later tccl release used by nodes is a node release; a tccl patch release that changes observable behaviour of an active language version is a bug, not an update.

### 2.2 Source-level changes in thecoin

| Change | Where | Effort |
|---|---|---|
| `CompileOptions { address_prefixes }` → `CompileOptions::network(hrp, language)` | `programs.rs::deploy`, `wallet/src/main.rs` | Small |
| Choose the language by height (§3) | `programs.rs::deploy` | Small |
| `vm::execute` → `vm::execute_with(..., ExecOptions { harden_v1 })` | `programs.rs::run`, `view` | Small |
| `FnKind::Upgrade` appears in ABIs | `api.rs`, explorer, wallet listing | Small |
| Typed rendering of records/enums: `abi::display_typed`, `abi::to_json_typed`, `abi::parse_arg_in` | `api.rs`, `node/src/rpc.rs`, wallet | Medium |
| `Host::enter_contract`, `leave_contract`, `contract_info` for `ChainHost` | `programs.rs` | Medium (§5) |
| Upgrade transactions and authority record | `tx.rs`, `execution.rs`, `state.rs`, wallet | Medium (§4) |
| Remove `crates/tccl` from the workspace; keep `docs/tccl` pointing to tccl.the-coin.cloud | repo | Small |

`Value`, `Type`'s version 1 variants, the `Value` Borsh layout and storage key layout (`[0,var]`, `[1,var,key]`, `[2,var]`, `[3,var,idx]`) are unchanged, so existing transactions, receipts and state decode as before.

## 3. Versions and activation

### 3.1 Rule

```text
deploy at height h:  language = if h >= TCCL_V2_HEIGHT { 2 } else { 1 }
invoke / view:       language = program.version (read from the stored code)
```

- No new field in `Deploy`: the height decides. A version 1 source deployed after activation compiles as version 2 (same functions, state and events; version 2 resource rules).
- The stored program records its language in its first two bytes, so replay never needs the height to execute an old contract.
- `TCCL_V2_HEIGHT` is a per-network constant in `ChainParams` set by the node release that implements this plan, after the governance software-upgrade proposal is approved; its `activation_height` becomes the constant. Regtest: height 1. Testnet first, mainnet only after the testnet period (§8).

### 3.2 Transaction format

Upgrades need new actions (§4). Adding `TxAction` variants changes the transaction format, so:

- `TX_VERSION` stays `1`, new variants are **appended** (Borsh tags 7, 8, …).
- Before `TCCL_V2_HEIGHT`, transactions using the new variants are invalid (rejected by mempool and block validation).
- Wallets refuse to build them before activation.

### 3.3 Language versions after 2

The same pattern repeats: frozen compiler per version, `TCCL_V3_HEIGHT`, programs self-describing. Never reuse a version number.

## 4. Upgrades (upgrade authority model)

Decision recorded with the project owner: contracts are upgradeable by an **upgrade authority** by default (the deployer), immediately, and can be made **final** forever.

### 4.1 State

New record, so `ProgramMeta` (`0x07`) keeps its encoding:

```text
0x0A | program address → ProgramAdmin { authority: Option<Address>, code_version: u32, language: u16, previous_code_hash: Hash32 }
```

- **Absent record = final.** Every contract deployed before activation is final forever; its code cannot change. This keeps a promise made to their users.
- Deploys at or after activation write `authority = Some(sender)`, unless the deploy sets the new flag bit `FLAG_FINAL_DEPLOY = 0x02` in `TxBody::flags` (unknown before activation, so old nodes and wallets reject it — correct).
- `code_version` starts at 1.

### 4.2 Transactions

```rust
TxAction::Upgrade {
    contract: Address,
    source: String,
    expected_code_hash: Hash32,   // BLAKE3 of the program being replaced (no race with another upgrade)
    args: Vec<Value>,             // for upgrade()
    max_fuel: u64,
    max_deposit: u64,
}
TxAction::SetUpgradeAuthority {
    contract: Address,
    new_authority: Option<Address>, // None = make final
    expected_code_hash: Hash32,
}
```

Execution of `Upgrade` (in a child overlay; any failure discards it, fee paid):

1. Load `ProgramAdmin`; fail unless `authority == Some(sender)`.
2. Fail unless `code_hash(current program) == expected_code_hash`.
3. Charge compile fuel (5 per source byte); compile with `CompileOptions { version: 2, state_order: current.states names, .. }`.
4. `tccl::upgrade::check(&old, &new)`; on error fail with the message.
5. Replace `0x08`, update `ProgramMeta.source_hash`, `deploy_txid` (the upgrade txid), `state_bytes` (code size difference), `ProgramAdmin.code_version += 1`, `previous_code_hash`.
6. Run `vm::execute_with(new, Mode::Upgrade, "upgrade", args, ctx{caller: sender, value: 0})`.
7. Settle the deposit with the existing rule; emit host log `ContractUpgraded(code_version, old_hash, new_hash)`.

`SetUpgradeAuthority` checks steps 1–2, writes the new authority (or `None`) and logs `UpgradeAuthorityChanged`.

`contract_info(addr)` for the VM returns `code_hash` and `upgradeable = authority.is_some()`.

### 4.3 Wallet, API and explorer

- `thecoin-wallet contract upgrade <address> <file> [args…]` shows the `upgrade::Report` and requires `--yes-upgrade` in non-interactive mode.
- `contract authority <address> <new|none>`; `none` asks for explicit confirmation ("this can never be undone").
- `GET /api/v1/program/{address}` adds `upgrade_authority`, `code_version`, `language`, `effects`, `access`, `records`, `enums`, `roles`, `modules`.
- The explorer shows a clear "upgradeable by …" or "final" badge on every contract page.

## 5. Calls between contracts in the node

### 5.1 Host implementation

`ChainHost` becomes a stack-aware host over **one overlay per transaction** (the existing child overlay):

```rust
struct ChainHost<'s, 'b, R> {
    state: &'s mut Overlay<'b, R>,
    stack: Vec<Frame>,                       // current contract = last
    metas: BTreeMap<Address, ProgramMeta>,   // loaded once per contract, written back on success
    programs: BTreeMap<Address, Arc<Program>>, // decode cache for this transaction
    logs: Vec<LogEntry>, touched: Vec<Address>, destroy_to: Option<Address>, fatal: Option<StateError>,
}
struct Frame { addr: Address }
```

- `storage_read/write/balance/send/emit/storage_items` use `stack.last().addr` and update that contract's meta (`state_bytes`, `storage_items`).
- `enter_contract(caller, callee, value)`: load meta + code (cache), return `None` if absent; if `value > 0` debit `caller` contract account and `credit` callee (`InsufficientBalance` on shortfall); push frame.
- `leave_contract`: pop.
- `destroy` is already rejected by the VM for nested frames.
- Events keep `LogEntry.contract` = the emitting contract (already in the receipt format).

### 5.2 Deposits and fees

- After success, for every contract whose `state_bytes` changed, apply the existing deposit rule; **the transaction sender pays or receives**, and the sum of growth deposits is limited by the transaction's single `max_deposit`.
- Fuel is one budget (`max_fuel`) shared by all frames; the fee formula is unchanged.
- `touched` includes every entered contract (for indexes and caches).

### 5.3 Rules enforced by the VM (already implemented in tccl)

| Rule | Error |
|---|---|
| Re-entrancy: a contract already on the stack | `re-entrant call: contract … is already running in this transaction` |
| Contract depth > 8, call depth > 16 | `contract call depth limit reached`, `call depth limit reached` |
| Signature mismatch | `interface mismatch: …` |
| Value to a non-payable function | `function does not accept TCN (not payable)` |
| Callee is a version 1 program | `not supported: calls into language version 1 contracts` |
| `destroy()` in a nested frame | `destroy() is only allowed when the contract is called directly by a transaction` |
| Views reaching actions | `state cannot be modified in a view` |

### 5.4 Mempool and simulation

- Keep the v0.2.0 rule: relayed transactions are not executed in the mempool (CPU DoS). Local API submissions and `tx/simulate` run the full call graph with the same host.
- The wallet's simulation (measured fuel × 1.3 + 5 000) automatically covers nested calls.

## 6. Fuel: version 2 schedule and the version 1 finding

### 6.1 Finding

Measured on an Intel Xeon E5-1620 v2 with `cargo test -p tccl --release --test perf -- --ignored --nocapture`:

| Workload (version 1) | ns per fuel | Full 50 M block |
|---|---|---|
| Copying a local `list[text]` of 4 000 items in a loop (`len(xs)`) | 1 389–2 109 | 69–105 s |
| Reading a large `list[text]` from a map in a loop | 1 425–1 656 | 71–83 s |
| Holding 1 000 × 32 KiB texts in a local list | memory 65.6 MB per transaction | — |

Cause: version 1 charges `size / 32` fuel for copying or decoding a value, independent of how many heap allocations it contains; `xs[i]` and `len(xs)` copy the whole list first.

Impact on The Coin today: a miner or attacker can publish blocks that take one to two minutes to validate on a 4-core server (≈ 100× the calibration), slowing propagation and validation across the network. We did not scan the chains for such contracts; testnet had about 21 blocks when this was written.

### 6.2 Version 2 fix (implemented)

- +4 fuel per heap allocation when copying a value; +1 per 32 bytes and +6 per allocation when decoding a stored value.
- `xs[i]` and `len(xs)` on a local list read without copying.
- 16 MiB memory limit across all running frames.
- Same workloads: 10–21 ns per fuel.

### 6.3 Options for version 1 contracts

`tccl::vm::ExecOptions { harden_v1 }` applies the version 2 resource rules to version 1 programs. It changes only fuel (and fails calls that exceed 16 MiB), never results of calls that finish.

| Option | Pros | Cons |
|---|---|---|
| **A. Enable `harden_v1` at `TCCL_V2_HEIGHT` (recommended)** | Closes the DoS for every contract with one activation | Fuel of some old calls rises; wallets re-simulate, so users only pay more for pathological copies |
| B. Separate earlier activation only for `harden_v1` | Fixes the DoS sooner, independent of version 2 review | Two coordinated upgrades |
| C. Do nothing for version 1 | No behaviour change for old contracts | The DoS stays open forever |

Before choosing, run the **fuel delta report**: replay every historical `Invoke` on testnet and mainnet with `harden_v1 = true` and list calls whose fuel changes, by how much, and whether they would exceed their original `max_fuel`.

Interim mitigation without consensus change: node operators may lower the local mempool/miner **policy** limit on `max_fuel` for version 1 invokes, and miners may refuse to include version 1 calls whose measured execution time per fuel exceeds a threshold. This does not protect validators from blocks produced by others.

## 7. Replay and compatibility guarantees

1. **Frozen compiler.** Nodes compile historical deploy sources with `language 1` forever. `src/v1` is never edited.
2. **Code check from state.** At upgrade time, a node tool re-compiles the source of every deployed contract (`deploy_txid` → source) with the new binary and compares with the bytes stored under `0x08`. Any difference aborts the release.
3. **Full replay.** Before a release, sync testnet and mainnet from genesis with the new binary and compare `state_root` at every height with the previous release (`thecoind --reindex --verify-roots`).
4. **Receipts replay.** Compare receipts (success, error strings, fuel used, logs, return values) for every historical transaction; error strings of version 1 are frozen.
5. **Differential CI.** Keep `crates/tccl-compat` pinned to the exact deployed engine of each network release.
6. **Encoding tests.** Program, `Value` and `ProgramMeta` vectors from v0.2.0 decode identically.

## 8. Tests before activation

| Level | What | Tool |
|---|---|---|
| Engine | Unit, integration, recipes, adversarial input, differential | `cargo test --workspace --release` (this repository) |
| Fuzzing | Parser/checker (both versions), program decoding, VM with random programs and hosts | `cargo fuzz` targets `compile_v1`, `compile_v2`, `decode_program`, `execute` — 72 h each without findings |
| Host | `ChainHost` calls: re-entrancy, depth, value transfer, deposits for several contracts, out of fuel in a nested frame, storage failure in a nested frame | `thecoin/crates/core/tests/execution.rs` |
| Transactions | Upgrade authorization, `expected_code_hash` races, incompatible upgrades, failing `upgrade()`, `SetUpgradeAuthority` to none, flags before/after activation | core tests |
| Consensus | Blocks at `H-1`, `H`, `H+1`; reorg across `H`; mixed old/new nodes before `H` | regtest multi-node (`node/tests/network.rs`) |
| Performance | Worst-case block time with version 2 and with `harden_v1` on 2-vCPU and 4-vCPU servers; memory per block | `tests/perf.rs` + node block benchmark |
| Tooling | Wallet deploy/invoke/upgrade/authority on regtest and testnet; explorer rendering; API typed values | wallet tests, explorer e2e |
| Public testnet | ≥ 4 weeks with the recipes deployed (token, pool, games, payments), a bug bounty and at least one external review of the VM and host changes | testnet |

## 9. Activation procedure

1. Merge the node changes behind `TCCL_V2_HEIGHT = u64::MAX` (disabled) and release.
2. Open a governance **software-upgrade** proposal with the release hash, this plan and the fuel delta report.
3. After approval, set testnet `TCCL_V2_HEIGHT` to the proposal's `activation_height`; release; operators upgrade during `activation_delay`.
4. Monitor on testnet: block validation time, memory, failed-call rates, fuel deltas, explorer/wallet errors.
5. Repeat the proposal for mainnet with the testnet results.
6. At activation, publish the language 2 documentation as current on the-coin.cloud and switch wallet defaults.

## 10. Recovery

| Situation | Response |
|---|---|
| Bug found **before** `H` | Release with a later `H` (or `u64::MAX`); no chain change |
| Consensus split at `H` (nodes disagree) | Operators stop mining; the correct behaviour is decided from the specification and differential tests; release a fix that keeps the canonical chain; nodes on the wrong branch reindex from the last common block |
| Security bug in version 2 **after** `H` | Emergency **soft fork**: new blocks reject new version 2 deploys, `Upgrade`, and invokes that would enter another contract (restrictive rules only, so old history stays valid); then a normal release with the fix and a new language version |
| Bug in a deployed upgradeable contract | Its upgrade authority upgrades it; users can check `is_final` and code hashes |
| Bug in a final contract | Cannot be changed by design; users migrate to a new contract |
| DoS through version 1 copies before a fix | Mempool/miner policy limits (§6.3), then option A or B |

Never rewrite history or edit stored programs to recover: every fix is a new rule from a height onward.

## 11. Decisions for the project owner

| Decision | Recommendation | Status |
|---|---|---|
| Upgrade model | Upgrade authority by default, final possible | **Decided** |
| `TCCL_V2_HEIGHT` for testnet and mainnet | Testnet after the node release; mainnet ≥ 4 weeks later | Open |
| Version 1 repricing (§6.3) | Option A, after the fuel delta report | Open |
| Calls into version 1 contracts | Not allowed (implemented) | Proposed |
| Default authority for deployments via the wallet | Deployer, with a prominent `--final` option | Proposed |
| External review of VM and host changes before mainnet | Required | Open |
| Code-signing certificate for Windows builds | Optional; unsigned builds with checksums for now | Open |

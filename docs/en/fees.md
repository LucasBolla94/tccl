# Fuel, fees and deposits

## Fuel

Every operation costs **fuel**. A transaction reserves a maximum (`max_fuel`); if execution needs more, the call fails and everything is reverted. Prices are calibrated so that about 20 ns of CPU on a 2-vCPU server correspond to one unit of fuel, which keeps a full block (50 000 000 fuel on mainnet by default) at about one second in the worst case.

| Operation | Fuel |
|---|---|
| Statement · expression | 2 · 1 |
| Per 32 bytes of values read, copied or combined | 1 |
| Function call | 20 |
| Storage read | 250 |
| Storage write or delete | 400 + 4 per byte of key and value |
| `send` · `emit` · `destroy` | 300 · 100 + 1 per byte · 1 000 |
| `sha256`, `blake3` | 60 + 20 per 64 bytes |
| `verify_ed25519` | 3 500 + 1 per 64 bytes |
| `ring_verify` | 5 000 + 10 000 per key |
| Deploy: compiling | 5 per byte of source |
| Invoke: loading the contract | 100 + 1 per 100 bytes of compiled code |

### Changes in language version 2

| Operation | Version 1 | Version 2 |
|---|---|---|
| Copying a value (reading a local, a constant) | 1 per 32 bytes | 1 per 32 bytes **+ 4 per heap allocation** (each text, bytes or list inside) |
| Decoding a value read from storage | included in the read | **+ 1 per 32 bytes + 6 per allocation** |
| `xs[i]` and `len(xs)` on a local list | copies the whole list | reads without copying |
| Call to another contract | — | 700 + 1 per 32 bytes of arguments + loading (100 + 1 per 100 bytes of code) |
| `mul_div`, `isqrt`, `pow` | — | 30 |
| `code_hash`, `is_contract`, `is_final` | — | 250 |
| Enum transition check | — | 1 per checked level |

Version 1 sources compiled as version 2 produce the same results, storage and events; their fuel can differ only where they copy lists or large values. The reasons are explained in [Security](security.md#v1-findings).

## Fees

The minimum fee of a transaction is

```text
fee = (base_fee + ⌈bytes × fee_per_kb ÷ 1000⌉ + ⌈max_fuel × fee_per_kfuel ÷ 1000⌉) × congestion
```

with The Coin's default parameters `base_fee = 1 000`, `fee_per_kb = 10 000` and `fee_per_kfuel = 1 000` motes. The congestion multiplier starts at 1× and adjusts with block usage; the part above 1× is burned. Wallet priorities `low | normal | high | urgent` pay 1×, 1.25×, ≥ 2× and ≥ 4× the minimum. Parameters are set by governance, so check the network's current values before relying on exact numbers.

- You pay for the fuel you **reserve**. The wallet reserves the measured fuel × 1.3 + 5 000.
- A **failed** transaction pays its fee; all of its effects are reverted.
- Views called through the API are free and do not create transactions.

`tccl run` and the playground print an estimate with the default parameters, for example:

```text
estimated on-chain cost (default parameters): fee 9247 motes for max_fuel 6497 and ~175 bytes · storage deposit +0 motes
```

## Storage deposits

Contract state is backed by a **refundable deposit**:

- The size of a contract is its compiled code plus every storage entry (key + value).
- Required deposit = ⌈size ÷ 1 000⌉ × `storage_deposit_per_kb` (default 100 000 motes, 0.001 TCN per started kB).
- A deploy or call that makes the state **grow** pays the missing deposit, up to the transaction's `max_deposit` (wallet default 1 TCN).
- A call that makes the state **shrink** receives `deposit × freed ÷ old size` back. Storing a default value deletes an entry, so clearing data is rewarded.
- `destroy(to)` pays the whole deposit and the balance to `to`.

The simulator shows deposits as estimates and does not deduct them from the fictitious balances.

## Measured performance

TCCL is a tree-walking interpreter written in Rust. Rust gives memory safety and predictable performance; it does not run contracts at native speed. What matters for the network is that fuel follows CPU time.

Measured with `cargo test -p tccl --release --test perf -- --ignored --nocapture` on an Intel Xeon E5-1620 v2 @ 3.70 GHz (the simulator's in-memory storage is faster than a node's disk, so storage rows are optimistic):

| Workload | Language | ns per fuel | Peak heap | 50 M-fuel block |
|---|---|---|---|---|
| Arithmetic loop | 1 · 2 | 16.4 · 17.6 | < 0.1 MB | 0.8 s · 0.9 s |
| Map writes | 1 · 2 | 1.4 · 1.2 | 0.4 MB | 0.1 s |
| BLAKE3 chain | 1 · 2 | 5.0 · 4.7 | < 0.1 MB | 0.2 s |
| Building records | 2 | 13.8 | < 0.1 MB | 0.7 s |
| Copying a list of 4 000 texts (adversarial) | 1 · 2 | **2 109** · 12.2 | 0.3 MB | **105 s** · 0.6 s |
| Decoding a stored list (adversarial) | 1 · 2 | **1 656** · 21.1 | 0.4 MB | **83 s** · 1.1 s |
| Holding large local lists (adversarial) | 1 · 2 | 10.1 · 6.4 | **65.6 MB** · 16.8 MB (limit) | 0.5 s · 0.3 s |

Your numbers will differ; run `tccl bench` for a quick check on your machine.

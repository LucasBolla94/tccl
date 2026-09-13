# Security

This page explains what the engine guarantees, what it cannot guarantee, and how to write contracts that stay safe.

## Guarantees of the engine

These are properties of the TCCL compiler and virtual machine. No contract, option or setting disables them.

| Guarantee | How |
|---|---|
| **Determinism** | No clock, floating point, randomness, threads or unordered iteration. The same call on the same state gives the same result, fuel and storage on every node. The compiler is part of consensus: every node compiles the deployed source itself. |
| **Checked arithmetic** | Every `int` operation is checked. Overflow and division by zero fail the call; `mul_div` avoids intermediate overflow. |
| **Fuel** | Every statement, expression, storage access, hash, signature and call costs fuel. When the transaction's fuel runs out, the call fails. A block has a fuel limit, so a block's execution time is bounded. |
| **Memory limits** | Values are at most 64 KiB, local lists 4 096 items, and in version 2 all running functions together hold at most 16 MiB. |
| **Bounded compilation** | Source size, nesting, expression depth and counts are limited, so hostile source cannot exhaust a node while compiling. |
| **Types** | Every value has one type; views cannot change anything; only payable functions receive TCN; maps exist only in storage. |
| **Atomic transactions** | Any failure reverts every change in every contract involved, every transfer and every event. |
| **No re-entrancy** (v2) | A contract running in a transaction cannot be called again during it. |
| **Explicit permissions** (v2) | `only` checks run before the function body and are listed in the interface. |
| **Transitions** (v2) | Enum transitions are checked on every store. |

## Failures and fees

When a call fails — `require`, overflow, out of fuel, a transition not allowed, an error in another contract — **everything the transaction did is reverted, and the fee is still paid**. The network did the work, and charging for it is what stops people from flooding it with failing calls.

- Wallets simulate every call on the current state and refuse to send one that would fail, so a failure after sending usually means the state changed between the simulation and the block (for example someone else bought the last item).
- You pay for the fuel you **reserve** (`max_fuel`), not only for the fuel used. The wallet reserves the measured fuel × 1.3 + 5 000.
- A failed call returns the value you attached and changes no deposit.

Details and numbers: [Fuel, fees and deposits](fees.md).

## Caller, origin and authorization

- Authorize with `caller` or `only`. Inside a call from another contract, `caller` is that contract.
- Never authorize with `origin`: it is the signer even when an unknown contract sits in between.
- Set owners and roles in `init()`; do not leave an unprotected setup action.

See [Permissions](permissions.md) and [Calling other contracts](calls.md).

## Money

- Pay out from **your own accounting**, not from `balance`: anyone can raise a contract's balance with a plain transfer.
- Update state before sending or calling (*checks → effects → calls*).
- Settle once: mark a payment as done (an enum status with transitions is ideal) before paying.
- Prefer **pull payments** — users withdraw what they are owed — over looping through a list of recipients.
- Multiply before dividing, or use `mul_div(a, b, c)`. Reject negative amounts.

## Denial of service

- Never loop over a list that other people can grow without limit: each iteration costs fuel and the call will eventually always fail.
- Keep per-user data in maps; iterate only over bounded lists.
- Remember that storage costs a refundable deposit paid by whoever makes the contract grow.

## Upgrades

An upgradeable contract is only as trustworthy as its **upgrade authority**, which can replace the code — including permission checks — at once. Check `is_final(addr)` before depending on another contract, and make your own contract final when it is stable. See [Upgrades](upgrades.md).

## Privacy and its limits {#privacy}

Everything stored on a public chain — state, arguments, events, balances — is visible to everyone, forever. `only` controls who can *change* data, not who can *read* it.

TCCL offers **linkable ring signatures** (`ring_verify`, bLSAG over Ristretto255, an established construction also used by Monero) for private payment pools such as [`private_pool.tccl`](recipes.md#more-recipes). A withdrawal proves "I own one of these N deposits" without saying which, and the key image stops the same deposit from being withdrawn twice. Know the limits:

- **Anonymity set.** You hide among the ring members (at most 64). Few deposits, or rings chosen carelessly, reveal a lot.
- **Amounts.** Pools use a fixed denomination; any other amount stands out.
- **Timing and behaviour.** Depositing and withdrawing close together, or in unusual patterns, links you.
- **Fees and gas.** Whoever pays the withdrawal fee is visible; use a relayer, which you must trust not to log you.
- **Metadata.** Your node, IP address and wallet behaviour are outside the contract's control.
- **Not zero-knowledge.** Ring signatures hide *which* member signed, not *that* one of them did. They do not hide amounts or program logic.
- **No new cryptography.** TCCL does not invent cryptographic schemes. Do not build your own from `sha256` and `blake3`; ask for a reviewed primitive instead.

## External data (oracles) {#external-data}

A contract cannot read the internet. Prices, results and real-world events enter only through transactions, so they are exactly as trustworthy as whoever provides them.

- Accept external data **signed** by known keys (`verify_ed25519`) and bind the message to `self`, the value, and a height or nonce so it cannot be replayed elsewhere.
- Prefer several independent signers and a threshold, and reject stale data using `height`.
- Plan what happens when the provider stops or lies: timeouts, an arbiter, refunds.
- A secret revealed in a transaction (for example to unlock a hash-locked payment) becomes public as soon as the transaction enters the mempool.

## Randomness {#randomness}

**Nothing on a blockchain is random.** Block height, hashes of previous data and transaction contents are known in advance or can be influenced by miners and users. A lottery that uses them can be predicted or manipulated.

Use **commit–reveal** between the parties who have something at stake: each commits to `sha256(secret + choice)`, then reveals; the contract checks the commitment and combines the secrets. The party that reveals last can refuse to reveal if it would lose, so give it a deadline and make refusing cost its stake. The recipe [`coin_flip.tccl`](recipes.md#coin-flip-game) does exactly this. For many participants, use a threshold of independent signers or a verifiable random source signed off-chain, and document the trust assumption.

## Front-running

Transactions wait in a public mempool before they are mined. Anyone can see a pending trade and try to act first.

- Protect trades with limits such as `min_out` (see [`pool.tccl`](recipes.md#exchange-pool)).
- Bind signatures to `self`, the recipient, amounts and fees.
- Use commit–reveal for auctions and games.

## Known issues in language version 1 {#v1-findings}

While building version 2 we measured the engine deployed on The Coin v0.2.0 and found that version 1 prices **copies of values by bytes, not by allocations**. A contract holding a local list of 4 000 short texts and calling `len(xs)` in a loop copies the whole list each time for about 500 fuel. On the reference machine (Intel Xeon E5-1620 v2) this costs about **1 400–2 100 ns per unit of fuel instead of ~20**, so a block filled with such calls could take **one to two minutes** to execute instead of about one second. Reading a large list stored in a map has the same problem (~1 650 ns per fuel), and a single transaction can hold about 65 MB of values.

- **Version 2 fixes it**: copies and decoded storage values pay per allocation, `xs[i]` and `len(xs)` no longer copy the list, and memory is limited to 16 MiB. The same workloads measure 12–21 ns per fuel.
- **Version 1 cannot be changed** without a consensus change, because past blocks must replay identically. The mitigation options — activating version 2 for new deployments, and a fuel repricing for version 1 calls at an activation height — are analysed in [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

The measurements are reproducible with `cargo test -p tccl --release --test perf -- --ignored --nocapture`.

## Checklist before real value {#checklist}

1. Every action that moves money or changes permissions has `only` or a `require` on `caller`.
2. Owners and roles are set in `init()`.
3. Every `require` has a scenario that makes it fail, and every success path has one that passes.
4. Payments settle once; state is updated before sending or calling.
5. No unbounded loops over lists others can grow.
6. Amounts are in motes; multiplications happen before divisions.
7. Signed messages include `self`, amounts, recipients and a height or nonce.
8. Randomness uses commit–reveal with deadlines; external data is signed and checked for staleness.
9. Dependencies are checked with `is_final` or `code_hash`.
10. The contract ran on testnet, the source is published, and someone other than the author reviewed it.
11. The upgrade authority is a multisig or governance contract — or the contract is final.

## Reporting a vulnerability

Please do not open a public issue for a vulnerability in the compiler, the virtual machine or the standard library. Follow [SECURITY.md](https://github.com/LucasBolla94/tccl/blob/main/SECURITY.md).

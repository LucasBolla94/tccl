# Calling other contracts

Language version 2 lets a contract call actions and views of other contracts. This is what makes tokens usable by exchanges, games usable by marketplaces and payments composable.

> **Network status.** Calls between contracts are part of language version 2. They run in `tccl`, scenarios and the playground today. The Coin v0.2.0 runs language version 1 and does not execute them yet; see [Versions](versions.md).

## Declaring and calling

```tccl
contract Payer

interface Token:
    action transfer(to: address, amount: int) -> bool
    action transfer_from(from: address, to: address, amount: int) -> bool
    view balance_of(who: address) -> int
    action deposit() payable

action pay(token: address, to: address, amount: int):
    require Token(token).transfer(to, amount), "transfer failed"

view held(token: address) -> int:
    return Token(token).balance_of(self)
```

- An `interface` lists the functions you need, with their exact signatures. It does not have to list every function of the other contract.
- `Token(addr)` uses an address through the interface. You can store it: `let t: Token = Token(addr)`, or declare `state token: address` and write `Token(token)`.
- The call returns the declared type. Ignore a result by calling on its own line.
- Signatures may use `int`, `bool`, `text`, `bytes`, `address` and lists of them. Records, enums and interfaces are local to a contract: pass their fields.

## Who is calling: `caller` and `origin`

When a user calls `Pool.swap` and the pool calls `Token.transfer_from`:

| Inside | `caller` | `origin` | `self` |
|---|---|---|---|
| `Pool.swap` | the user | the user | the pool |
| `Token.transfer_from` | **the pool** | the user | the token |

**Authorize with `caller`, never with `origin`.** If a token checked `origin`, any contract the user interacts with could move the user's tokens. `origin` exists for logging and for rejecting calls that do not come directly from a person (`require caller == origin`), which also blocks every contract, including multisig wallets — use it rarely.

Because the token sees the pool as `caller`, users first **approve** the pool on the token (`token.approve(pool, amount)`), and the pool moves tokens with `transfer_from(user, self, amount)`.

## Sending TCN with a call

```tccl
action buy(shop: address, price: int) payable:
    require value == price, "send the price"
    Shop(shop).deposit() with value price
```

- `with value <amount>` moves motes **from the calling contract's balance** to the called contract, before the called function runs. The called contract sees it as `value`.
- The interface must mark the function `payable`, and the called function must really be payable; otherwise the call fails with `function does not accept TCN (not payable)`.
- `send(to, amount)` is different: it pays an address and **never runs code**, even if the address is a contract.

## Return values

The called function's `return` value comes back typed as the interface declares. At run time the engine checks that the called function has the same kind (`action` or `view`), parameter types and return type; a difference fails with `interface mismatch: 'f' is action (text) -> int in the called contract but (int) -> int in the interface`.

## Everything or nothing

A transaction is **atomic**. If anything fails anywhere — a `require` in a contract three calls deep, an overflow, running out of fuel, a transition not allowed — then:

- every storage change in **every** contract involved is discarded;
- every TCN transfer, including `with value` and the value the user attached, is undone;
- no events are recorded;
- the transaction fee is still paid, because the network executed it.

There is no `try`/`catch` in language version 2: a contract cannot swallow a failure of another contract and continue in a half-updated state. Design flows so that a failure is a clear "no". Wallets simulate calls first and do not send calls that would fail.

## Re-entrancy is impossible

A contract that is already running in the transaction **cannot be called again** during it. If `A` calls `B` and `B` tries to call `A`, the call fails with `re-entrant call` and everything is reverted. A contract calling itself through an interface is rejected the same way. This protection is part of the engine and cannot be disabled.

This removes the classic attack where a callee re-enters a half-updated contract. It is still good practice to update your own state before calling out (*checks → effects → calls*), because it keeps the code easy to reason about:

```tccl
action withdraw(token: address, amount: int):
    require deposits[caller] >= amount, "not enough"
    deposits[caller] -= amount                         # effects first
    require Token(token).transfer(caller, amount), "transfer failed"   # then the call
```

Callbacks (a token notifying the contract that called it) are therefore not possible. Use pull patterns: the other contract exposes a view or an action that you call.

## Views and read-only calls

- A `view` can call views of other contracts. It cannot call actions — that is a compile error.
- Everything reached from a view runs read-only: an attempt to change state fails with `state cannot be modified in a view`.

## Checking what you call

```tccl
init(token_address: address):
    require is_contract(token_address), "not a contract"
    require is_final(token_address), "the token must not be upgradeable"
    token = token_address
    token_code = code_hash(token_address)
```

- `is_contract(addr)` — whether a contract exists there.
- `is_final(addr)` — the contract has no upgrade authority, so its code can never change ([Upgrades](upgrades.md)).
- `code_hash(addr)` — BLAKE3 of its compiled code; compare it with a hash you reviewed.

## Limits and costs

| | |
|---|---|
| Contracts on the call stack | 8 (`contract call depth limit reached`) |
| Function call depth, across all contracts | 16 |
| Fuel | One budget for the whole transaction, shared by every contract |
| Memory | 16 MiB of values across every running function |
| Cost of a call | 700 fuel + 1 per 32 bytes of arguments + loading 100 + 1 per 100 bytes of code |
| `destroy()` | Only when the contract is called directly by a transaction |
| Language version 1 contracts | Cannot be called by other contracts (`not supported: calls into language version 1 contracts`): they were written when `caller` was always a signer |

## Example: an exchange pool

The recipe [`pool.tccl`](recipes.md#exchange-pool) is a constant-product exchange that holds two tokens. `add_liquidity` and `swap` update the reserves and then call `transfer_from`/`transfer` on the token contracts. If a trader has not approved enough tokens, the token's `require` fails and the whole swap — including the reserve update — is reverted. Its scenario tests exactly that.

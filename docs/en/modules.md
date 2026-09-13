# Modules and the standard library

Language version 2 separates two ideas that are easy to confuse:

| | Module | Deployed contract |
|---|---|---|
| What it is | Source code reused at compile time | A program living at an address on the chain |
| Written as | `module name` (a section or a standard module) | `contract Name` |
| Has an address, balance, deposit | No | Yes |
| Deployed or upgraded on its own | Never | Yes |
| How you use it | `use std.token`, then `token.mint(...)` | Through an `interface` and a call |
| Where its state lives | Inside the contract that uses it | In its own storage |
| Who can change its state | Only the module's own functions | Only its own code |
| Cost | Its code becomes part of the contract (size, compile fuel) | Calling it costs a contract call |

In short: **a module is copied into your contract; a contract is called.** Choose a module to reuse code, choose another contract to share state or funds with other applications.

## Using a standard module

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

- `use std.token` compiles the standard token module into `CloudCoin`. `use std.token as coin` picks another name.
- The module's **helpers** are called through its name: `token.mint(to, amount)`.
- The module's **actions and views** become part of the contract's interface: `CloudCoin` now has `transfer`, `approve`, `transfer_from`, `balance_of` and the rest. A name clash with the contract is a compile error.
- The module's **state** is stored inside the contract under qualified names (`token.balances`). The contract can read it (`token.total_supply`) but only the module's functions can change it — this keeps the module's rules (balances never negative, supply equals the sum of balances) true no matter what the contract does.
- Module records and enums are named `token.Record` from the contract.

## Your own modules

Put `module` sections at the end of the file:

```tccl
contract App
use mathx

view average(a: int, b: int) -> int:
    return mathx.mean(a, b)

module mathx
fn mean(a: int, b: int) -> int:
    return (a + b) / 2
```

Or keep the module in `mathx.tccl` next to the contract (starting with `module mathx`). `tccl check`, `tccl run` and `tccl test` find it automatically, and `tccl bundle app.tccl -o deploy.tccl` writes the single file you deploy: a deployment always carries one source file.

A module can declare constants, records, enums, interfaces, events, state, roles, `fn` helpers, actions and views. It cannot declare `init()` or `upgrade()` (export a `fn setup(...)` instead) and cannot `use` other modules. Every `module` section must be used.

## Standard modules are frozen

The standard library ships inside the compiler and is **frozen per language version**: `std.token` in language version 2 will always compile to the same code. The compiled program records the BLAKE3 hash of every module source it includes (`tccl abi` shows it). Improvements go into a new language version.

## `std.token` — fungible tokens

State: `token_name`, `token_symbol`, `token_decimals`, `total_supply`, `balances: map[address, int]`, `allowances`.
Events: `Transfer(from, to, amount)` (mint: from the zero address; burn: to it), `Approval(owner, spender, amount)`.

| Exported | Kind | Behaviour |
|---|---|---|
| `transfer(to, amount) -> bool` | action | Moves the caller's tokens; `to` cannot be the zero address |
| `approve(spender, amount) -> bool` | action | Sets how much `spender` may move from the caller |
| `transfer_from(from, to, amount) -> bool` | action | Moves `from`'s tokens using the caller's allowance |
| `balance_of(who)`, `allowance(owner, spender)`, `total_supply_of()`, `name()`, `symbol()`, `decimals()` | views | |

| Helper | Use it for |
|---|---|
| `token.setup(name, symbol, decimals)` | Once, in `init()`; name 1–64 bytes, symbol 1–12, decimals 0–18 |
| `token.mint(to, amount)` | Creating tokens — you decide who may call it (e.g. `only minter`) |
| `token.burn(owner, amount)` | Destroying tokens |
| `token.move(from, to, amount)` | Custom transfer rules |

> Approvals follow the usual pattern and share its classic race: to change an allowance from N to M, set it to 0 first, then to M.

## `std.items` — unique items

For collectibles, game objects, tickets and certificates. Each item has an `id`, an `owner`, a `kind` (1–32 bytes), `metadata` (≤ 1 024 bytes) and the `created` height, stored as the record `items.Item`.
Events: `ItemCreated(id, owner, kind)`, `ItemTransferred(id, from, to)`, `ItemApproved(id, owner, spender)`, `ItemBurned(id, owner)`.

| Exported | Kind | Behaviour |
|---|---|---|
| `transfer_item(id, to)` | action | Owner only; clears the approval |
| `approve_item(id, spender)` | action | Owner lets one address move the item |
| `take_item(id, to)` | action | The approved address moves the item |
| `item(id) -> items.Item`, `owner_of(id)`, `items_owned_by(who)`, `item_count()` | views | |

Helpers: `items.create(to, kind, metadata) -> int`, `items.move_item(id, to)`, `items.burn_item(id)`. Metadata is public; store a hash or a link if the content is large or private.

## `std.payments` — conditional payments

Holds TCN until a condition is met. Each payment is a `payments.Payment` record with a `payments.Status` that can only go `Pending → Released` or `Pending → Refunded`.
Events: `PaymentCreated(id, payer, payee, amount)`, `PaymentReleased(id, payee, amount, reason)`, `PaymentRefunded(id, payer, amount, reason)`.

| Exported | Who | Condition |
|---|---|---|
| `create_payment(payee, arbiter, release_after, refund_after, hashlock) payable -> int` | anyone | Locks the attached TCN. Use the zero address, `0` or empty bytes to disable arbiter, heights or hash lock |
| `release_payment(id)` | payer or arbiter | Pays the payee |
| `claim_payment(id)` | payee | After `release_after` (if set) |
| `reveal_payment(id, secret)` | anyone | If `sha256(secret) == hashlock`: pays the payee (a hash-locked payment) |
| `refund_payment(id)` | payee or arbiter at any time; payer after `refund_after` | Returns the TCN to the payer |
| `payment(id)`, `payment_status(id)`, `locked_total()` | views | |

Helpers: `payments.create(...)`, `payments.pay_out(id, reason)`, `payments.pay_back(id, reason)`.

> A revealed secret is public once the transaction is in the mempool. Hash locks protect *who gets paid*, not the secrecy of the secret. See [Security](security.md#external-data).

See the [recipes](recipes.md) for complete, tested contracts using each module.

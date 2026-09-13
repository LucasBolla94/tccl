# Language reference

This page describes language version 2. Everything marked **v2** is new in version 2; everything else also exists in version 1 and behaves identically there.

## File layout

```tccl
contract Shop                      # 1. header: the first line of code

use std.token                      # 2. modules (v2)

const MAX_NAME: int = 64           # 3. declarations, in any order
state owner: address
state prices: map[text, int]
event Sold(item: text, buyer: address, price: int)
role clerk                         # v2

init():                            # 4. functions
    owner = caller
    grant clerk to caller

action buy(item: text) payable:
    require prices.has(item), "unknown item"
    require value == prices[item], "wrong price"
    send(owner, value)
    emit Sold(item, caller, value)

view price_of(item: text) -> int:
    return prices[item]
```

- The first line of code is `contract Name`. One file holds one contract; `module` sections may follow it ([Modules](modules.md)).
- Blocks are defined by indentation with **spaces only** (a tab is a compile error). Lines inside `( )` and `[ ]` may continue on the next line. Comments start with `#`.
- Declarations can appear in any order. Functions can call helpers declared later. A constant can only use constants declared above it.
- A contract needs at least one `init`, `action` or `view`.
- A name is declared once per contract and cannot shadow a local or a built-in name.

**Keywords:** `contract const state event init action view fn payable let if elif else while for in break continue return require send emit destroy pass true false and or not`

**Contextual words (v2):** `module use as record enum interface role only grant revoke to from with value upgrade`. They are keywords only where the grammar expects them, so version 1 contracts that use them as names (for example a parameter called `to`) keep compiling.

**Reserved names:** `caller value balance height self TCN int bool text bytes address list map len sha256 blake3 to_bytes to_text to_int min max abs slice verify_ed25519 ring_verify address_of zero_address range`. The v2 built-ins `mul_div isqrt pow code_hash is_contract is_final` and the context value `origin` are not reserved: if a contract declares one of these names, its own declaration wins.

## Types

| Type | Default | Literal | Notes |
|---|---|---|---|
| `int` | `0` | `42`, `-7`, `1_000_000` | Signed 128-bit, checked arithmetic |
| `bool` | `false` | `true`, `false` | |
| `text` | `""` | `"hi\n"` | UTF-8; `len` counts bytes; escapes `\n \t \" \\` |
| `bytes` | empty | `0xdead_beef` | Even number of hex digits |
| `address` | zero address | `address("tc1…")` | 20 bytes; the prefix must match the network |
| `list[T]` | `[]` | `[1, 2, 3]` | `T` is any type except a map |
| `map[K, V]` | empty | — | State variables only; `K` is a scalar or an enum |
| record **v2** | every field at its default | `Order(buyer: caller, amount: 5)` | Named, typed fields |
| enum **v2** | first variant | `Status.Open` | Named situations, optional transitions |
| interface **v2** | zero address | `Token(addr)` | An address used to call another contract |

Scalars (`int bool text bytes address`) can be compared with `==` and `!=`, used as map keys and given initial values in `state`. Enums can also be compared, used as map keys and given initial values. Records and lists cannot be compared with `==`.

There are no implicit conversions and no decimals. Amounts are integers in **motes**: 1 TCN = `TCN` = 100 000 000 motes. Write `5 * TCN` in code; the `5tcn` notation is only for command-line and scenario arguments.

## Constants and state

```tccl
const FEE_BP: int = 25                        # computed at compile time
const TREASURY: address = address("tc1yfugpgq45fs8xe80x4jam2j2w2kqhk9qnp2umy")
state owner: address                          # default: zero address
state name: text = "Cloud Token"              # scalars and enums may have a constant initial value
state phase: Phase = Phase.Open               # v2
state holders: list[address]                  # stored item by item
state balances: map[address, int]
state orders: map[int, Order]                 # v2: records in maps
```

Constants may use literals, earlier constants, operators, enum variants and `address("…")`.

Whole state lists and maps cannot be assigned or copied: work with their items. **Default values are never stored:** assigning `0`, `false`, `""`, empty bytes or list, the zero address, the first enum variant or a record whose fields are all defaults deletes the storage entry, so `m.has(k)` becomes `false` and the storage deposit is refunded.

## Functions

| Kind | Header | Notes |
|---|---|---|
| `init` | `init(params) [payable]:` | Optional, at most one; runs at deployment; no return value |
| `action` | `action name(params) [-> T] [payable] [only …]:` | Called by transactions; may return a value (receipt `return_value`) |
| `view` | `view name(params) -> T:` | Free read-only query; must return a value |
| `fn` | `fn name(params) [-> T] [only …]:` | Private helper, only callable from code |
| `upgrade` **v2** | `upgrade(params) [only …]:` | Runs once when an upgrade installs this code ([Upgrades](upgrades.md)) |

- Version 2 accepts `payable` before or after the return type. Version 1 requires `-> T payable`.
- Only `action` and `init` can be `payable`. Sending TCN to any other function fails with `function does not accept TCN (not payable)`.
- A view cannot assign state, `send`, `emit`, `destroy`, read `value`, call an `fn` that does any of these, or call an action of another contract.
- A function with a return type must return on every path: its last statement is a `return`, or an `if`/`else` whose branches all end with `return`.
- Entry points cannot be called from code. Recursion is allowed up to a call depth of 16.
- `only` lists roles or address state variables; see [Permissions](permissions.md).

## Statements

```tccl
let fee: int = amount / 100              # locals: type and value are mandatory
balances[to] += amount - fee             # = += -= *=  (+= also joins text and bytes)
orders[id].status = Status.Paid          # v2: record fields, in locals and in storage
require caller == owner, "only the owner"
if amount > 100 * TCN:
    pass
elif amount > 0:
    queue.push(to)
else:
    require false, "amount must be positive"
for i in range(0, 3):                    # start … end-1
    continue
for who in queue:                        # local or state list
    break
while len(queue) > 10:
    queue.pop()
send(to, fee)                            # pay from the contract balance
emit Paid(to, fee)
grant clerk to who                       # v2
revoke clerk from who                    # v2
```

- `require cond[, message]` stops the call and **reverts every effect** when `cond` is false (`requirement failed: message`; without a message: `requirement at line N failed`).
- `send(to, amount)` fails with `invalid amount` (≤ 0) or `insufficient contract balance`. It never runs code at the receiver.
- `emit Event(args…)` — the arguments must match the declared fields. Events go into the receipt.
- `destroy(to)` — actions only: deletes the contract and pays its balance and storage deposit to `to`. Every list item and map entry must be removed first. Not allowed when another contract called this one.
- `return [value]`, `break`, `continue`, `pass`.
- An expression alone on a line must be a function call, a call to another contract or `xs.pop()`.
- The index of a compound assignment is evaluated once: `m[next_id()] += 1` calls `next_id()` one time.

## Operators

| Precedence (low → high) | Operators | Operand types |
|---|---|---|
| 1 | `or` | bool, short-circuit |
| 2 | `and` | bool, short-circuit |
| 3 | `not` | bool |
| 4 | `==` `!=` · `<` `<=` `>` `>=` | same scalar or enum type · int |
| 5 | `+` `-` | int; `+` also text + text, bytes + bytes |
| 6 | `*` `/` `%` | int |
| 7 | unary `-` | int |
| 8 | `x[i]` `x.field` `x.m(…)` `f(…)` | |

Overflow fails with `integer overflow`. `/` and `%` by zero fail. `/` truncates toward zero and `%` takes the sign of the left operand (`-7 / 2 == -3`, `-7 % 2 == -1`). Chained comparisons (`a < b < c`) are rejected. Conditions must be `bool`: there is no truthiness.

An interface value can be compared with an address: `Token(a) == b`.

## Lists and maps

| Operation | Local list | State list |
|---|---|---|
| literal `[a, b]`, `[]` | ✓ | — |
| `xs[i]`, `xs[i] = v`, `xs[i] += v` | ✓ | ✓ |
| `xs.push(v)` | ✓ | ✓ |
| `xs.pop()` (statement or expression) | — | ✓ |
| `len(xs)` / `xs.len()` | ✓ | ✓ |
| `for x in xs:` | ✓ | ✓ |
| assign or copy the whole list | ✓ | — |

Local lists hold at most 4 096 items and 65 536 bytes. Maps support `m[k]` (the default if absent), `m[k] = v`, `m[k] += v`, `m.has(k)` and `m.remove(k)`. Maps cannot be iterated: keep a list of keys if you need one. To change a list stored in a map, copy it to a local, change it and store it back.

## Records **v2** {#records}

A record groups named, typed fields.

```tccl
record Order:
    buyer: address
    item: text
    price: int
    status: Status

state orders: map[int, Order]

action place(item: text) -> int:
    next_id += 1
    orders[next_id] = Order(buyer: caller, item: item, price: prices[item], status: Status.Placed)
    return next_id

view buyer_of(id: int) -> address:
    return orders[id].buyer
```

- Build a record with **every field named exactly once**: `Order(buyer: …, item: …, price: …, status: …)`. Missing, unknown or repeated fields are compile errors.
- Read fields with `.`: `o.price`, `orders[id].buyer`. Assign them in locals and in storage: `o.price = 5`, `orders[id].status = Status.Paid`, `orders[id].price += 1`.
- A field can be any type except a map, including another record or a list. A record cannot contain itself.
- Records can be parameters, return values, event fields, state variables and map or list values. They cannot be map keys or compared with `==`.
- A record whose fields are all defaults is the default record: storing it deletes the entry.
- Records declared in a module are named `module.Record` from the contract.
- In the ABI, arguments are written `{buyer: tc1…, item: "lamp", price: 5, status: Placed}`.

## Enums and transitions **v2** {#enums}

An enum lists named situations. The first variant is the default.

```tccl
enum Color: Red, Green, Blue           # inline form, no transitions

enum Status:                           # block form with allowed transitions
    Placed -> Paid, Cancelled
    Paid -> Shipped, Refunded
    Shipped -> Delivered
    Delivered
    Cancelled
    Refunded
```

- Write variants as `Status.Paid` (or `module.Status.Paid`). `to_text(s)` returns the variant name, e.g. `"Paid"`.
- Enums can be compared with `==`/`!=`, used as map keys, constants, initial state values, record fields and parameters (arguments are written as the variant name: `Paid`).
- If **any** variant lists `-> next, …`, transitions are enforced for that enum. A variant without an arrow is final: nothing can follow it.
- Transitions are checked **whenever a value is stored**: a state variable, a map value, a state list item, or a field of a record stored in any of these. Changing `orders[id].status` from `Placed` to `Delivered` fails with `transition not allowed: Status cannot go from Placed to Delivered`, and the whole call is reverted.
- Staying in the same variant is always allowed, so updating other fields of a record does not trip the check.
- A new map entry starts from the default (first) variant; pushing onto a state list is checked from the default too.
- Local variables are not checked: only what is stored matters.
- An upgrade may add variants at the end, never reorder or remove them ([Upgrades](upgrades.md)).

## Roles and `only` **v2**

```tccl
role manager
state treasurer: address

init():
    grant manager to caller

action set_price(item: text, price: int) only manager:
    prices[item] = price

action pay_out(to: address, amount: int) only manager, treasurer:
    send(to, amount)

view is_manager(who: address) -> bool:
    return manager.has(who)
```

- `role name` declares a set of addresses stored by the contract.
- `grant name to <address>` and `revoke name from <address>` change it and emit `RoleGranted(role, account, by)` or `RoleRevoked(role, account, by)` automatically.
- `name.has(address)` tells whether an address holds the role.
- `only a, b` on an action, `fn` or `upgrade()` lets the call continue only if `caller` holds role `a`, or is the address stored in `b` (an address state variable), and so on. Otherwise it fails with `only a or b can call 'f'`.
- The rules appear in the interface (`tccl check`, `tccl abi`, the playground) so users see who can do what.

Details and patterns: [Permissions](permissions.md).

## Interfaces and calls **v2**

```tccl
interface Token:
    action transfer(to: address, amount: int) -> bool
    action transfer_from(from: address, to: address, amount: int) -> bool
    view balance_of(who: address) -> int
    action buy() payable

action pay(token: address, to: address, amount: int):
    require Token(token).transfer(to, amount), "transfer failed"

action shop(t: address):
    Token(t).buy() with value 2 * TCN

view held(token: address) -> int:
    return Token(token).balance_of(self)
```

- `Name(address)` treats an address as a contract with that interface; `let t: Token = Token(a)` stores it.
- Inside the called contract, `caller` is **the calling contract**; `origin` is the account that signed the transaction.
- `with value <amount>` sends TCN from the calling contract's balance; the interface must mark the function `payable`.
- Interface signatures use `int bool text bytes address` and lists of them. At run time the called function must have exactly the same kind, parameter types and return type, or the call fails with `interface mismatch`.
- A failure in the called contract reverts the whole transaction. A contract that is already running cannot be called again (re-entrancy is always rejected).

Everything about calls — identity, transfers, atomicity, limits — is in [Calling other contracts](calls.md).

## Context values

| Name | Type | Meaning |
|---|---|---|
| `caller` | `address` | Who called this function: the signer, or the calling contract. Zero address inside views |
| `origin` **v2** | `address` | The account that signed the transaction, even inside a call from another contract |
| `value` | `int` | Motes sent with this call (not available in views) |
| `balance` | `int` | This contract's balance in motes, including `value` |
| `height` | `int` | Block height (about one block per minute) |
| `self` | `address` | This contract's address |
| `TCN` | `int` | 100 000 000 |

## Built-in functions

| Function | Result | Extra fuel |
|---|---|---|
| `len(list \| text \| bytes)` | `int` | — |
| `min(a, b)`, `max(a, b)`, `abs(a)` | `int` | — |
| `to_text(int \| enum)` | decimal text, or the variant name | — |
| `to_bytes(int \| address \| text \| bool \| bytes)` | `bytes` (int: 16 bytes big-endian; address: 20; bool: 1) | — |
| `to_int(bytes)` | unsigned big-endian `int` from ≤ 15 bytes | — |
| `slice(bytes, start, end)` | `bytes` | — |
| `sha256(bytes \| text)`, `blake3(bytes \| text)` | 32 bytes | 60 + 20 per 64 bytes |
| `verify_ed25519(pk, msg, sig)` | `bool` (false for malformed input) | 3 500 + 1 per 64 bytes |
| `ring_verify(ring: list[bytes], msg, sig, key_image)` | `bool` — linkable ring signature (bLSAG, Ristretto255), ≤ 64 keys | 5 000 + 10 000 per key |
| `address_of(pk: bytes)` | `address` of a 32-byte public key | — |
| `zero_address()` | `address` | — |
| `address("tc1…")` | compile-time address literal | — |
| `mul_div(a, b, c)` **v2** | ⌊a × b ÷ c⌋ toward zero, no intermediate overflow | 30 |
| `isqrt(x)` **v2** | ⌊√x⌋ for x ≥ 0 | 30 |
| `pow(base, exp)` **v2** | checked integer power, exp ≥ 0 | 30 |
| `code_hash(addr)` **v2** | BLAKE3 of a contract's compiled code, empty if not a contract | 250 |
| `is_contract(addr)` **v2** | `bool` | 250 |
| `is_final(addr)` **v2** | `true` if a contract has no upgrade authority | 250 |

```tccl
let msg: bytes = blake3(to_bytes(self) + to_bytes(caller) + to_bytes(amount))
require verify_ed25519(pk, msg, sig), "bad signature"
let out: int = mul_div(amount_in * 9_970, reserve_out, reserve_in * 10_000 + amount_in * 9_970)
```

## Limits

| Limit | Value |
|---|---|
| Source code (one file, including module sections) | 48 000 bytes |
| Compiled program | 262 144 bytes |
| Fuel per transaction · per block (mainnet default) | 10 000 000 · 50 000 000 |
| Function call depth | 16 |
| Contracts on the call stack **v2** | 8 |
| Memory held by running functions **v2** | 16 MiB per transaction |
| Nesting of blocks, parentheses, types | 32 |
| Operators of one precedence level in one expression · expression depth | 64 · 128 |
| Functions · state variables · locals per function | 256 · 256 · 1 024 |
| Records · fields per record · enums · variants · interfaces **v2** | 128 · 64 · 128 · 256 · 64 |
| Modules per contract **v2** | 16 |
| Text, bytes or list value | 65 536 bytes |
| Items in a local list | 4 096 |
| Events per call · arguments per call | 64 · 32 |
| Ring size | 64 |

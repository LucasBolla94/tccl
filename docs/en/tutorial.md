# Your first contract

This tutorial takes about ten minutes. You will install `tccl`, create a contract, find and fix an error, test it with a scenario, simulate transactions and learn how to deploy it to The Coin.

> Everything in steps 1 to 6 happens on your computer. Nothing is signed or published, and no wallet or recovery phrase is involved.

## 1. Install

**Linux**

```sh
curl -fsSL https://tccl.the-coin.cloud/install.sh | sh
```

**Windows** (PowerShell)

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://tccl.the-coin.cloud/install.ps1 | iex"
```

Both scripts download the latest release from GitHub and verify its SHA-256 checksum. You can also use the MSI installer or build from source — see [Tools](tools.md#install). Check the installation:

```sh
tccl version
```

Prefer not to install anything? Open the [playground](/playground/): it runs the same compiler and virtual machine in your browser.

## 2. Create a project

```sh
tccl new tip-counter
cd tip-counter
```

This creates two files: `contract.tccl` and `contract.scenario`. Open `contract.tccl`:

```tccl
# TipCounter: a starting point. Edit it, then run `tccl check contract.tccl` and `tccl test`.
contract TipCounter

state owner: address
state count: int

event Increased(by: address, total: int)

init():
    owner = caller

action increment(amount: int):
    require amount > 0, "amount must be positive"
    count += amount
    emit Increased(caller, count)

action reset() only owner:
    count = 0

view get() -> int:
    return count
```

Read it from top to bottom:

- Lines starting with `#` are comments.
- `contract TipCounter` names the contract. One file holds one contract.
- `state` variables are stored on the chain. `owner` starts as the zero address and `count` as `0`.
- `event Increased(...)` declares something the contract can announce. Events go into the transaction receipt, where wallets and explorers read them.
- `init()` runs once, at deployment. `caller` is whoever sent the transaction, so the deployer becomes the owner.
- An `action` is called by a transaction and may change state. `require` stops the call — and reverts everything — when its condition is false.
- `only owner` lets only the address stored in `owner` call `reset`.
- A `view` is a free, read-only query. It must return a value.

## 3. Check it

```sh
tccl check contract.tccl
```

```text
✔ TipCounter compiles (language 2, 438 bytes of source, 376 bytes compiled)
  state: owner: address, count: int
  init init()
  action increment(amount: int)
  action reset() only owner
  view get() -> int
  can: change state, emit events
```

`tccl check` compiles exactly like the network does and prints the interface: every entry point, who may call it and what the contract can do.

## 4. Break it on purpose

Change `return count` to `return cout` and check again:

```text
error[C006]: unknown name 'cout'
  --> contract.tccl:21:12
   |
21 |     return cout
   |            ^
  = why: Every name must be declared before use: a local 'let', a parameter, a const, a state variable or a built-in context value (caller, value, balance, height, self, origin).
  = fix: Check the spelling or declare it, e.g. 'let total: int = 0'. (did you mean 'count'?)
```

Every error has a position, a code you can look up with `tccl explain C006` or in the [error reference](errors.md), an explanation and a fix. Add `--lang pt` or `--lang es` for Portuguese or Spanish. Undo the change before continuing.

## 5. Test it with a scenario

`contract.scenario` is a small test script:

```scenario contract.scenario
deploy contract.tccl as app --from owner
call app increment 5 --from bob
expect ok
expect event Increased
view app get
expect result 5
call app reset --from bob
expect fail "only owner"
call app reset --from owner
expect ok
```

Run every scenario in the folder:

```sh
tccl test
```

```text
✔ ./contract.scenario (5 checks)
5 checks passed, 0 failed, 1 scenario file(s)
```

Accounts such as `owner` and `bob` are fictitious; each starts with 1 000 000 TCN. Scenarios are the fastest way to prove that permissions and failures behave as you intend. The full syntax is in [Tools](tools.md#scenarios).

## 6. Simulate transactions

`tccl run` keeps a simulated chain in `tccl-state.json`, so you can explore step by step:

```sh
tccl run contract.tccl deploy
tccl run contract.tccl call increment 5 --from bob
tccl run contract.tccl call reset --from bob
tccl run contract.tccl view get
tccl run contract.tccl state
```

```text
event Increased(by: tcr14vqs008pe0yx022r6pneffl2mnn0x2lkzh7452, total: 5)
ok · fuel used: 1152 · height: 3 · bob balance: 100000000000000 motes
estimated on-chain cost (default parameters): fee 9247 motes for max_fuel 6497 and ~175 bytes · storage deposit +0 motes
FAILED[R002]: requirement failed: only owner can call 'reset' · fuel used: 281 (all changes reverted)
  = A 'require' was false; the call stops and every effect is reverted.
  = Read the message: it says which condition was not met.
```

The simulator prints the fuel used and an **estimate** of the fee and storage deposit with The Coin's default parameters. Addresses start with `tcr1` because simulated accounts are not mainnet (`tc1`) or testnet (`tct1`) accounts.

## 7. Deploy to The Coin

Deploying uses the node software's wallet, [`thecoin-wallet`](https://the-coin.cloud/docs.html), not `tccl`. `tccl` never handles keys.

> **Language version on the network.** The Coin v0.2.0 runs language version 1. The template above uses `only`, which is version 2. To deploy today, replace `only owner` with `require caller == owner, "only the owner"` and check with `tccl check --language 1 contract.tccl`.

Always start on testnet:

```sh
thecoin-wallet --network testnet contract deploy contract.tccl
thecoin-wallet --network testnet contract invoke <address> increment 5
thecoin-wallet --network testnet contract view <address> get
```

The wallet simulates every call before sending it and refuses calls that would fail. See [Tools](tools.md#testnet) and [Fuel, fees and deposits](fees.md).

## Next steps

- Change the examples in the [playground](/playground/) — try `orders.tccl` and `pool.tccl`.
- Read the [language reference](language.md).
- Before handling real value, read [Security](security.md) and write a scenario for every `require`.

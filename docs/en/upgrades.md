# Upgrades

Deployed code sometimes needs fixes. TCCL uses an **upgrade authority**: each contract has at most one address allowed to replace its code, and that address can give its power away or renounce it forever. Nothing is upgraded implicitly — every upgrade is a transaction signed by the authority.

> **Network status.** Upgrades are defined by language version 2 and work in `tccl`, scenarios and the playground. The Coin v0.2.0 has no upgrade transaction: contracts deployed there today cannot change. Activating upgrades requires a protocol change described in [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

## The model

| | |
|---|---|
| At deployment | The deployer becomes the upgrade authority, unless the contract is deployed **final** (`--final`) |
| Upgrading | The authority submits new source code; it replaces the code **immediately** if it is compatible |
| Changing the authority | The authority can hand it to another address, for example a multisig or a governance contract |
| Renouncing | The authority can set it to none: the contract becomes **final**, forever |
| Final contracts | Can never be upgraded again |

This is flexible — a team can fix a bug the same day — and that flexibility is also the risk: **whoever holds the authority controls the contract**, including its funds and permissions. Users should treat an upgradeable contract as trusting its authority.

## What users and other contracts can check

- `tccl run … state` and the playground show the upgrade authority and the code version.
- Other contracts can require immutable dependencies: `require is_final(token), "token must be final"`.
- `code_hash(addr)` identifies the exact code; compare it with the hash of the code you reviewed.

Good practice for projects that keep an authority:

- hold it in a multisig or governance contract, not a single hot key;
- announce upgrades and publish the new source before submitting them;
- make the contract final once it is stable.

## Compatibility rules

Storage survives an upgrade, so the new code must read the old data the same way. `tccl` checks, before anything runs:

- every state variable of the old code still exists **with the same name and a compatible type** — you cannot remove or retype a variable (stop using it instead);
- new state variables are added after the existing ones (the compiler keeps each existing variable in its storage slot automatically, whatever order you write them in);
- records stored in state keep **exactly the same fields** in the same order;
- enums stored in state keep their variants in order; new variants may only be **added at the end**;
- the language version does not go backwards.

Anything else — functions, events, constants, roles' rules, new records — can change. The upgrade report lists added state, added, removed and changed functions, and notes such as *"the new code can send TCN (the old code could not)"*.

## `upgrade()` — migrating data

```tccl
contract Counter

state count: int
state last_caller: address
state step: int                      # new in this version

upgrade():
    step = 10                        # runs once, inside the upgrade transaction

action increment(amount: int):
    count += amount * step
```

- `upgrade(params)` runs once when the upgrade installs the code, called by the authority (`caller`). It may take parameters and use `only`.
- If it fails, the whole upgrade is reverted and the old code stays.
- It cannot be called later: it is not an action.

## Doing it

```sh
tccl run counter.tccl deploy --from dev               # dev is the upgrade authority
tccl run counter_v2.tccl upgrade --from dev
tccl run counter.tccl authority @multisig --from dev  # hand over the authority
tccl run counter.tccl authority none --from multisig  # make the contract final
```

In a scenario:

```scenario upgrade.scenario
deploy counter.tccl as counter --from dev
upgrade counter counter_v2.tccl --from bob
expect fail
upgrade counter counter_v2.tccl --from dev
expect ok
authority counter none --from dev
```

The complete tested recipe is [Upgrading a contract](recipes.md#upgrading-a-contract).

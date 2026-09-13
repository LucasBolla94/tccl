# Tested recipes

Each recipe is a complete contract from the [`examples/`](https://github.com/LucasBolla94/tccl/tree/main/examples) folder with a scenario that checks its behaviour, including the failures. Every scenario runs in continuous integration (`cargo test` and `tccl test examples`), so the code on this page is known to work with this release.

Run one yourself:

```sh
git clone https://github.com/LucasBolla94/tccl && cd tccl/examples
tccl test counter.scenario
```

Or open the contract in the [playground](/playground/): the scenario is loaded into the *Scenario* tab.

## Counter

State, actions, views and events — the smallest useful contract. Language version 1, deployable on The Coin today.

{{example:counter.tccl}}

{{scenario:counter.scenario}}

## Tip jar

A payable action, an owner-only withdrawal and a view returning several numbers. Version 1.

{{example:tip_jar.tccl}}

{{scenario:tip_jar.scenario}}

## Token with a minter role

The standard token module plus a role. `transfer`, `approve`, `transfer_from` and the views come from `std.token`; the contract only decides who may mint.

{{example:cloud_coin.tccl}}

{{scenario:cloud_coin.scenario}}

## Orders with records, transitions and roles

A typed `Order` record, a `Status` enum whose allowed transitions stop impossible changes (a shipped order cannot be cancelled), and two roles.

{{example:orders.tccl}}

{{scenario:orders.scenario}}

## Exchange pool {#exchange-pool}

A constant-product exchange (DEX) for two token contracts with a 0.30 % fee. It shows interfaces, `caller` inside another contract (traders approve the pool), `mul_div` and `isqrt`, slippage protection with `min_out`, and atomicity: a swap without enough allowance fails in the token and reverts the pool's reserve update.

{{example:pool.tccl}}

{{scenario:pool.scenario}}

> Real exchanges also need protection against price manipulation within one block when other contracts use the pool's price. Do not use `quote` as a price oracle.

## Coin flip game {#coin-flip-game}

A two-player game without fake randomness: the host commits to a hidden choice, the guest guesses, the host reveals. A host who refuses to reveal loses after 20 blocks. See [Randomness](security.md#randomness).

{{example:coin_flip.tccl}}

{{scenario:coin_flip.scenario}}

## Tickets as unique items

Tickets issued by an organizer with `std.items`, limited capacity, transfers and check-in (burning the ticket).

{{example:tickets.tccl}}

{{scenario:tickets.scenario}}

## Conditional payments

All actions come from `std.payments`: a hash-locked payment released by whoever reveals the secret, and a payment the buyer can take back after a deadline.

{{example:deals.tccl}}

{{scenario:deals.scenario}}

## Upgrading a contract {#upgrading-a-contract}

The counter above, upgraded to a second version by its upgrade authority. The new state variable is initialized by `upgrade()`, the old values are kept, strangers cannot upgrade, and a final contract can never change again.

{{example:counter_v2.tccl}}

{{scenario:upgrade.scenario}}

## Escrow with an arbiter

Roles expressed with `require`, deadlines, a dispute and `destroy`. Version 1.

{{example:escrow.tccl}}

{{scenario:escrow.scenario}}

## More recipes {#more-recipes}

These version 1 contracts are tested in `crates/tccl/tests/examples.rs`:

| Contract | Shows |
|---|---|
| [`shop.tccl`](../../examples/shop.tccl) | Every kind of declaration, payable, helpers |
| [`token.tccl`](../../examples/token.tccl) | A token written by hand: maps, allowances, composite keys |
| [`crowdfund.tccl`](../../examples/crowdfund.tccl) | Deadlines and refunds |
| [`poll.tccl`](../../examples/poll.tccl) | List arguments, state lists, bounded loops |
| [`savings.tccl`](../../examples/savings.tccl) | Time locks and storage refunds |
| [`treasury.tccl`](../../examples/treasury.tccl) | M-of-N approvals |
| [`names.tccl`](../../examples/names.tccl) | Text keys, validation, expiry |
| [`private_pool.tccl`](../../examples/private_pool.tccl) | Ring signatures for private payments (see [Privacy](security.md#privacy)) |

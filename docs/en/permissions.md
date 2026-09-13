# Permissions

Who may do what is the most important question in any contract. Language version 2 makes the answer explicit in the code and visible in the interface.

## Roles

```tccl
role manager
role shipper

init():
    grant manager to caller

action hire_shipper(who: address) only manager:
    grant shipper to who

action fire_shipper(who: address) only manager:
    revoke shipper from who

action ship(id: int) only shipper:
    orders[id].status = Status.Shipped
```

- `role name` declares a set of addresses. A contract may declare several roles; a module may declare its own.
- `grant name to <address>` adds an address; `revoke name from <address>` removes it. Both are statements and emit `RoleGranted(role, account, by)` or `RoleRevoked(role, account, by)`, so the history of permissions is in the receipts.
- `name.has(address)` returns whether the address holds the role.
- A role is not a value: it cannot be assigned, passed or returned.
- Nobody holds a role until the contract grants it. Grant the first administrator in `init()`.

## `only`

```tccl
state owner: address
state treasurer: address
role auditor

action withdraw(to: address, amount: int) only owner, treasurer:
    send(to, amount)

fn audit_log(note: text) only auditor:
    emit Audited(caller, note)
```

- `only a, b, …` goes at the end of the function header. The call continues if `caller` holds any listed role, or equals any listed **address state variable**. Otherwise it fails with `requirement failed: only owner or treasurer can call 'withdraw'` and nothing changes.
- It works on actions, `fn` helpers and `upgrade()`. It is not allowed on views (free queries have no verified caller) or on `init()` (no role exists yet).
- The check runs before the first statement of the function, so no work is done for an unauthorized caller.
- `tccl check`, `tccl abi` and the playground list the rules next to each function, e.g. `action withdraw(to: address, amount: int) only owner, treasurer`.

`only owner` is exactly `require caller == owner, "only owner can call '…'"` written in a way tools can read. Version 1 contracts use the `require` form.

## Patterns

**Transferable ownership**

```tccl
state owner: address
state pending_owner: address

action offer_ownership(to: address) only owner:
    pending_owner = to

action accept_ownership() only pending_owner:
    owner = pending_owner
    pending_owner = zero_address()
```

The two steps prevent handing the contract to a mistyped address.

**Never lock yourself out.** Revoking the last administrator is permanent. Keep a count if it matters:

```tccl
role admin
state admins: int

action add_admin(who: address) only admin:
    require not admin.has(who), "already an admin"
    grant admin to who
    admins += 1

action remove_admin(who: address) only admin:
    require admin.has(who), "not an admin"
    require admins > 1, "keep at least one admin"
    revoke admin from who
    admins -= 1
```

**Roles for contracts.** A role can be granted to a contract address. Inside a call from that contract, `caller` is the contract, so `only operator` works for contracts too — for example a game contract allowed to mint items.

**Multiple approvals.** For decisions that need M of N people, store approvals per proposal and act when the count reaches M — see [`treasury.tccl`](recipes.md#more-recipes).

## Mistakes to avoid

- **Authorizing with `origin`.** Always use `caller` (or `only`). `origin` is the signer of the transaction even when an unknown contract is in between.
- **Unprotected initialization.** Set owners and grant roles in `init()`, which runs once at deployment. A `setup` action anyone can call first is a takeover.
- **Forgetting that views are public.** `only` does not hide data: everything stored on the chain can be read by anyone.
- **Trusting the upgrade authority blindly.** An upgradeable contract's authority can replace its code, including its permission checks. See [Upgrades](upgrades.md).

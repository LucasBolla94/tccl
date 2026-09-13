# Error reference

> This page is generated from the compiler's catalog (`tccl docs errors`). The same explanations appear in `tccl check`, `tccl explain <code>` and the playground.

Error *messages* are always in English because they are part of the compiler's behaviour (and, for language version 1, of chain history). The code, explanation and fix are translated.

## Compile errors

A compile error is shown as `file:line:column`, with the code, a suggested fix and often a “did you mean” hint. A contract that does not compile is never deployed; on the network, a deployment transaction whose source fails to compile is rejected by the wallet before sending, and pays its fee if it is mined anyway.

### C001 — Tab character

Blocks are defined by indentation and TCCL only accepts spaces, so the meaning of a line never depends on editor settings.

**Fix:** Replace tabs with 4 spaces.

Messages include: `tabs are not allowed`

### C002 — Indentation

After a line ending with ':' the next lines must be indented more than it; a block ends when the indentation returns to an outer level.

**Fix:** Indent the body of every block with the same number of spaces (4 is customary); use 'pass' for an empty block.

Messages include: `indentation does not match`, `unexpected indentation`, `an indented block`, `empty block`

### C003 — Unfinished text

A text literal must start and end with " on the same line. Escapes: \n \t \" \\.

**Fix:** Close the quote, or escape inner quotes as \".

Messages include: `unterminated`, `unknown escape`

### C004 — Invalid literal

Integers are decimal (underscores allowed) and fit in 128 bits; bytes are written 0x followed by an even number of hex digits. There are no decimals: amounts are integers in motes.

**Fix:** Write amounts as integers, e.g. 5 * TCN or 250_000_000.

Messages include: `invalid number literal`, `integer literal out of range`, `hex bytes literal`, `invalid hex literal`

### C005 — Contract layout

A deployable file starts with 'contract Name' and declares at least one init, action or view. Modules may follow as 'module name' sections.

**Fix:** Put 'contract MyContract' on the first line of code.

Messages include: `'contract <Name>'`, `only one contract`, `a contract needs at least one`, `only contains modules`

### C006 — Unknown name

Every name must be declared before use: a local 'let', a parameter, a const, a state variable or a built-in context value (caller, value, balance, height, self, origin).

**Fix:** Check the spelling or declare it, e.g. 'let total: int = 0'.

Messages include: `unknown name`, `unknown variable`

### C007 — Unknown type

Types are int, bool, text, bytes, address, list[T], map[K, V] (state only) and the records, enums and interfaces you declare.

**Fix:** Use one of the built-in types or declare the record/enum.

Messages include: `unknown type`

### C008 — Function call

Calls must name a built-in or a 'fn' helper with the right number and types of arguments.

**Fix:** Check the name and the parameters in the declaration.

Messages include: `unknown function`, `takes 0 argument`, `takes 1 argument`, `takes 2 argument`, `takes 3 argument`, `argument(s),`

### C009 — Type mismatch

TCCL never converts types implicitly: an operation or assignment needs exactly the declared type. Conditions must be bool.

**Fix:** Convert explicitly with to_text, to_bytes or to_int, or change the declared type.

Messages include: `expected int, found`, `expected bool, found`, `expected text, found`, `expected bytes, found`, `expected address, found`, `cannot add`, `arithmetic needs`, `cannot compare`, `ordering comparisons`, `need bool operands`, `compound assignment is not defined`, `expected a `

### C010 — Missing return

A function that declares '-> type' must end with 'return', or with an if/else whose every branch returns.

**Fix:** Add a final 'return <value>'.

Messages include: `must return a`, `must return on every path`, `does not return a value`

### C011 — Views only read

A view is a free query that anyone can run without a transaction, so it cannot change state, send TCN, emit events, read 'value' or call actions.

**Fix:** Turn it into an 'action', or move the change into an action.

Messages include: `a view cannot`, `changes state, sends TCN or emits events`, `not available in a view`, `view cannot call the action`

### C012 — Name already used

Each name is declared once per contract and cannot shadow built-in names (caller, value, len, ...).

**Fix:** Choose a different name.

Messages include: `already declared`, `is reserved`, `reserved name`, `duplicate`

### C013 — Maps and state lists

Maps exist only as state variables and are used item by item; state lists and maps cannot be copied or assigned as a whole.

**Fix:** Use m[key], m.has(key), m.remove(key), xs[i], xs.push(v), len(xs).

Messages include: `maps can only be used as state`, `map keys must be`, `cannot assign a whole`, `is a map;`, `is a state list`

### C014 — Constants

Constants are computed at compile time from literals, other constants, arithmetic, enum variants and address("..."), and never change.

**Fix:** Use a state variable for values that change.

Messages include: `is a constant`, `is not a constant`, `constant values can only`, `constant expression`, `constants must be`

### C015 — Receiving TCN

Only functions marked 'payable' (actions and init) can receive TCN; a call to another contract can attach TCN with 'with value' only if the interface marks it payable.

**Fix:** Add 'payable' to the header: action buy() payable:

Messages include: `can be payable`, `not payable in its interface`, `'with value'`, `views cannot receive TCN`

### C016 — Entry points are not helpers

init, actions and views are called by transactions and queries; code can only call 'fn' helpers (and other contracts through interfaces).

**Fix:** Move the shared logic into a 'fn' and call it from both places.

Messages include: `is an entry point`

### C017 — Statement

A line must do something: assign, call a function, control flow. Comparisons cannot be chained (use 'and').

**Fix:** Assign the result with 'let' or remove the line.

Messages include: `does nothing as a statement`, `outside of a loop`, `range() can only`, `chained comparisons`

### C018 — Records

A record groups named, typed fields. Build it with every field named, e.g. Order(buyer: caller, amount: 5); read with order.amount.

**Fix:** List every field once with 'name: value'.

Messages include: `record`, `has no field`, `needs every field`, `named fields`, `contains itself`

### C019 — Enums and transitions

An enum lists named situations. Lines like 'Open -> Paid, Cancelled' declare which changes are allowed; any other change fails when stored.

**Fix:** Use Enum.Variant and list allowed next variants after '->'.

Messages include: `variant`, `'->'`

### C020 — Roles and permissions

'only' restricts who may call an action: members of a role (role admin; grant admin to x) or the address in a state variable.

**Fix:** Declare 'role name' and grant it in init(): grant name to caller.

Messages include: `'only`, `unknown role`, `is a role`

### C021 — Interfaces

An interface describes actions and views of another contract. Calls use it: Token(addr).transfer(to, 5). Signatures use int, bool, text, bytes, address and lists.

**Fix:** Declare the function in the interface exactly as the other contract defines it.

Messages include: `interface`

### C022 — Modules

A module is reusable source code compiled into the contract ('use std.token' or a 'module name' section). It has no address; its state can only be changed by its own functions.

**Fix:** Call the module's helpers: token.mint(to, amount).

Messages include: `module`, `std.`

### C023 — Upgrades

An upgrade keeps the storage: state variables cannot be removed or change type, records keep their fields and enums keep their variants in order.

**Fix:** Keep old variables (you may stop using them) and add new ones.

Messages include: `upgrade`, `was removed`, `storage slot`

### C024 — Limit reached

The compiler bounds source size, nesting and counts so that every node can compile any contract quickly and safely.

**Fix:** Split long expressions with 'let' and large contracts into modules or several contracts.

Messages include: `too many`, `too large`, `too deep`, `too deeply`, `too long`, `nested too`

### C025 — Address literal

address("...") takes a valid bech32m address of the network being compiled for (tc1 mainnet, tct1 testnet, tcr1 regtest).

**Fix:** Copy the address again and check its prefix.

Messages include: `address literal`, `address prefix`, `invalid address`

### C026 — Syntax

The line does not follow the grammar at the marked position.

**Fix:** Compare with the language reference or an example.

Messages include: `expected`

## Runtime errors

A runtime error stops the call. **Every effect of the transaction is reverted** — storage in every contract involved, TCN sent, events and the value attached to the call. The transaction fee is still paid, because the network did the work. Wallets simulate calls first and refuse to send a call that would fail.

| Code | Error | Meaning | Fix |
|---|---|---|---|
| R001 | `out of fuel` | The call used all the fuel reserved by the transaction. Every change is reverted; the fee is still paid. | Bound loops, avoid iterating lists others can grow, or reserve more fuel. |
| R002 | `requirement failed: …` | A 'require' was false; the call stops and every effect is reverted. | Read the message: it says which condition was not met. |
| R003 | `integer overflow` | Integer overflow or division by zero; TCCL arithmetic is always checked. | Check inputs with require before dividing; use mul_div for a × b ÷ c. |
| R004 | `division by zero` | Integer overflow or division by zero; TCCL arithmetic is always checked. | Check inputs with require before dividing; use mul_div for a × b ÷ c. |
| R005 | `index I out of bounds (length N)` | An index was negative or ≥ the length (also pop on an empty list). | require i >= 0 and i < len(xs) |
| R006 | `value too large` | A value exceeded 64 KiB, a list 4 096 items, the events limit, or running functions held more than 16 MiB. | Keep data in state maps instead of large local lists. |
| R007 | `call depth limit reached` | More than 16 nested function calls or 8 contracts on the call stack. | Replace recursion with loops; flatten chains of contract calls. |
| R008 | `unknown function 'f'` | The function does not exist, is not of the kind called (action/view), or got wrong arguments. | Check the contract interface (tccl abi). |
| R009 | `function 'f' cannot be called this way` | The function does not exist, is not of the kind called (action/view), or got wrong arguments. | Check the contract interface (tccl abi). |
| R010 | `wrong arguments: …` | The function does not exist, is not of the kind called (action/view), or got wrong arguments. | Check the contract interface (tccl abi). |
| R011 | `function does not accept TCN (not payable)` | TCN was sent to a function that is not 'payable'. The value is returned by the revert. | Send no value, or mark the action payable. |
| R012 | `state cannot be modified in a view` | A view (or a contract reached from a view) tried to change something. | Use an action. |
| R013 | `invalid amount` | send() needs a positive amount and enough contract balance. | Track what the contract owes in state and check it before sending. |
| R014 | `insufficient contract balance` | send() needs a positive amount and enough contract balance. | Track what the contract owes in state and check it before sending. |
| R015 | `contract cannot be destroyed while it still has storage (N entries)` | destroy() needs empty maps and lists, and is not allowed when another contract called this one. | Remove every item first, and call destroy directly from a transaction. |
| R016 | `host error: …` | The call failed and every change it made was reverted. | Read the message for details. |
| R017 | `internal type error: …` | The call failed and every change it made was reverted. | Read the message for details. |
| R018 | `re-entrant call: contract … is already running in this transaction` | A contract already running in this transaction was called again. The network always blocks this to prevent re-entrancy attacks. | Design flows so calls go one way (A → B), and pass data as arguments or return values. |
| R019 | `contract call depth limit reached` | More than 16 nested function calls or 8 contracts on the call stack. | Replace recursion with loops; flatten chains of contract calls. |
| R020 | `no contract at …` | There is no contract at the address, or its function does not match the interface. | Check the address and declare the interface exactly like the target's ABI. |
| R021 | `interface mismatch: …` | There is no contract at the address, or its function does not match the interface. | Check the address and declare the interface exactly like the target's ABI. |
| R022 | `memory limit reached (16777216 bytes)` | A value exceeded 64 KiB, a list 4 096 items, the events limit, or running functions held more than 16 MiB. | Keep data in state maps instead of large local lists. |
| R023 | `transition not allowed: E cannot go from A to B` | The enum declares which changes are allowed and this one is not listed. | Follow the declared path (e.g. Paid → Shipped → Delivered) or add the transition. |
| R024 | `destroy() is only allowed when the contract is called directly by a transaction` | destroy() needs empty maps and lists, and is not allowed when another contract called this one. | Remove every item first, and call destroy directly from a transaction. |
| R025 | `not supported: …` | The call failed and every change it made was reverted. | Read the message for details. |

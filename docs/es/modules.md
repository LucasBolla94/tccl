# Módulos y biblioteca estándar

La versión 2 del lenguaje separa dos ideas fáciles de confundir:

| | Módulo | Contrato desplegado |
|---|---|---|
| Qué es | Código fuente reutilizado al compilar | Un programa que vive en una dirección de la cadena |
| Se escribe como | `module nombre` (una sección o un módulo estándar) | `contract Nombre` |
| Tiene dirección, saldo, depósito | No | Sí |
| Se despliega o actualiza por sí solo | Nunca | Sí |
| Cómo se usa | `use std.token`, luego `token.mint(...)` | Mediante una `interface` y una llamada |
| Dónde vive su estado | Dentro del contrato que lo usa | En su propio almacenamiento |
| Quién puede cambiar su estado | Solo las funciones del propio módulo | Solo su propio código |
| Costo | Su código pasa a formar parte del contrato (tamaño, combustible de compilación) | Llamarlo cuesta una llamada entre contratos |

En resumen: **un módulo se copia dentro de su contrato; un contrato se llama.** Elija un módulo para reutilizar código y otro contrato para compartir estado o fondos con otras aplicaciones.

## Usar un módulo estándar

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

- `use std.token` compila el módulo de token estándar dentro de `CloudCoin`. `use std.token as coin` elige otro nombre.
- Las **funciones auxiliares** del módulo se llaman con su nombre: `token.mint(to, amount)`.
- Las **actions y views** del módulo pasan a formar parte de la interfaz del contrato: `CloudCoin` ahora tiene `transfer`, `approve`, `transfer_from`, `balance_of` y las demás. Un choque de nombres con el contrato es un error de compilación.
- El **estado** del módulo se guarda dentro del contrato con nombres calificados (`token.balances`). El contrato puede leerlo (`token.total_supply`), pero solo las funciones del módulo pueden cambiarlo — así las reglas del módulo (saldos nunca negativos, oferta igual a la suma de saldos) se mantienen hagan lo que haga el contrato.
- Los records y enums del módulo se llaman `token.Record` desde el contrato.

## Sus propios módulos

Ponga secciones `module` al final del archivo:

```tccl
contract App
use mathx

view average(a: int, b: int) -> int:
    return mathx.mean(a, b)

module mathx
fn mean(a: int, b: int) -> int:
    return (a + b) / 2
```

O guarde el módulo en `mathx.tccl` junto al contrato (empezando con `module mathx`). `tccl check`, `tccl run` y `tccl test` lo encuentran solos, y `tccl bundle app.tccl -o deploy.tccl` escribe el archivo único que despliega: un despliegue siempre lleva un solo archivo de código.

Un módulo puede declarar constantes, records, enums, interfaces, eventos, estado, roles, auxiliares `fn`, actions y views. No puede declarar `init()` ni `upgrade()` (exporte un `fn setup(...)`) y no puede usar `use` con otros módulos. Toda sección `module` debe usarse.

## Los módulos estándar están congelados

La biblioteca estándar viene dentro del compilador y está **congelada por versión del lenguaje**: `std.token` en la versión 2 siempre compilará al mismo código. El programa compilado registra el hash BLAKE3 de cada módulo que incluye (`tccl abi` lo muestra). Las mejoras van a una nueva versión del lenguaje.

## `std.token` — tokens fungibles

Estado: `token_name`, `token_symbol`, `token_decimals`, `total_supply`, `balances: map[address, int]`, `allowances`.
Eventos: `Transfer(from, to, amount)` (emisión: desde la dirección cero; quema: hacia ella), `Approval(owner, spender, amount)`.

| Exportado | Tipo | Comportamiento |
|---|---|---|
| `transfer(to, amount) -> bool` | action | Mueve los tokens de quien llama; `to` no puede ser la dirección cero |
| `approve(spender, amount) -> bool` | action | Fija cuánto puede mover `spender` de quien llama |
| `transfer_from(from, to, amount) -> bool` | action | Mueve tokens de `from` usando la autorización de quien llama |
| `balance_of(who)`, `allowance(owner, spender)`, `total_supply_of()`, `name()`, `symbol()`, `decimals()` | views | |

| Auxiliar | Úselo para |
|---|---|
| `token.setup(name, symbol, decimals)` | Una vez, en `init()`; nombre 1–64 bytes, símbolo 1–12, decimales 0–18 |
| `token.mint(to, amount)` | Crear tokens — usted decide quién puede llamarlo (por ejemplo `only minter`) |
| `token.burn(owner, amount)` | Destruir tokens |
| `token.move(from, to, amount)` | Reglas de transferencia propias |

> Las autorizaciones siguen el patrón habitual y comparten su carrera clásica: para cambiar una autorización de N a M, fíjela primero en 0 y luego en M.

## `std.items` — objetos únicos

Para coleccionables, objetos de juego, entradas y certificados. Cada objeto tiene un `id`, un `owner`, un `kind` (1–32 bytes), `metadata` (≤ 1 024 bytes) y la altura `created`, guardados en el record `items.Item`.
Eventos: `ItemCreated(id, owner, kind)`, `ItemTransferred(id, from, to)`, `ItemApproved(id, owner, spender)`, `ItemBurned(id, owner)`.

| Exportado | Tipo | Comportamiento |
|---|---|---|
| `transfer_item(id, to)` | action | Solo el dueño; borra la aprobación |
| `approve_item(id, spender)` | action | El dueño deja que una dirección mueva el objeto |
| `take_item(id, to)` | action | La dirección aprobada mueve el objeto |
| `item(id) -> items.Item`, `owner_of(id)`, `items_owned_by(who)`, `item_count()` | views | |

Auxiliares: `items.create(to, kind, metadata) -> int`, `items.move_item(id, to)`, `items.burn_item(id)`. Los metadatos son públicos; guarde un hash o un enlace si el contenido es grande o privado.

## `std.payments` — pagos condicionales

Retiene TCN hasta que se cumple una condición. Cada pago es un record `payments.Payment` con un `payments.Status` que solo puede ir de `Pending → Released` o `Pending → Refunded`.
Eventos: `PaymentCreated(id, payer, payee, amount)`, `PaymentReleased(id, payee, amount, reason)`, `PaymentRefunded(id, payer, amount, reason)`.

| Exportado | Quién | Condición |
|---|---|---|
| `create_payment(payee, arbiter, release_after, refund_after, hashlock) payable -> int` | cualquiera | Bloquea el TCN adjunto. Use la dirección cero, `0` o bytes vacíos para desactivar árbitro, alturas o bloqueo por hash |
| `release_payment(id)` | pagador o árbitro | Paga al beneficiario |
| `claim_payment(id)` | beneficiario | Después de `release_after` (si está fijado) |
| `reveal_payment(id, secret)` | cualquiera | Si `sha256(secret) == hashlock`: paga al beneficiario (pago bloqueado por hash) |
| `refund_payment(id)` | beneficiario o árbitro en cualquier momento; pagador después de `refund_after` | Devuelve el TCN al pagador |
| `payment(id)`, `payment_status(id)`, `locked_total()` | views | |

Auxiliares: `payments.create(...)`, `payments.pay_out(id, reason)`, `payments.pay_back(id, reason)`.

> Un secreto revelado es público en cuanto la transacción entra en el mempool. Los bloqueos por hash protegen *quién cobra*, no el secreto en sí. Vea [Seguridad](security.md#external-data).

Vea las [recetas](recipes.md) con contratos completos y probados que usan cada módulo.

# Llamar a otros contratos

La versión 2 del lenguaje permite que un contrato llame actions y views de otros contratos. Es lo que hace que los tokens sirvan a los exchanges, que los juegos sirvan a los mercados y que los pagos se puedan combinar.

> **Estado en la red.** Las llamadas entre contratos forman parte de la versión 2 del lenguaje. Ya funcionan en `tccl`, los escenarios y el playground. The Coin v0.2.0 ejecuta la versión 1 y todavía no las ejecuta; vea [Versiones](versions.md).

## Declarar y llamar

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

- Una `interface` enumera las funciones que necesita, con sus firmas exactas. No tiene que enumerar todas las funciones del otro contrato.
- `Token(dirección)` usa una dirección mediante la interfaz. Puede guardarla: `let t: Token = Token(addr)`, o declarar `state token: address` y escribir `Token(token)`.
- La llamada devuelve el tipo declarado. Para ignorar el resultado, haga la llamada sola en su línea.
- Las firmas pueden usar `int`, `bool`, `text`, `bytes`, `address` y listas de ellos. Los records, enums e interfaces son locales de cada contrato: pase sus campos.

## Quién llama: `caller` y `origin`

Cuando un usuario llama a `Pool.swap` y el pool llama a `Token.transfer_from`:

| Dentro de | `caller` | `origin` | `self` |
|---|---|---|---|
| `Pool.swap` | el usuario | el usuario | el pool |
| `Token.transfer_from` | **el pool** | el usuario | el token |

**Autorice con `caller`, nunca con `origin`.** Si un token verificara `origin`, cualquier contrato con el que el usuario interactúe podría mover sus tokens. `origin` existe para registros y para rechazar llamadas que no vienen directamente de una persona (`require caller == origin`), lo que también bloquea a cualquier contrato, incluidas las billeteras multifirma — úselo poco.

Como el token ve al pool como `caller`, los usuarios primero **aprueban** al pool en el token (`token.approve(pool, amount)`), y el pool mueve los tokens con `transfer_from(usuario, self, amount)`.

## Enviar TCN con una llamada

```tccl
action buy(shop: address, price: int) payable:
    require value == price, "send the price"
    Shop(shop).deposit() with value price
```

- `with value <monto>` mueve motes **desde el saldo del contrato que llama** al contrato llamado, antes de que se ejecute la función llamada. El contrato llamado lo ve como `value`.
- La interfaz debe marcar la función como `payable`, y la función llamada debe serlo de verdad; si no, la llamada falla con `function does not accept TCN (not payable)`.
- `send(to, amount)` es distinto: paga a una dirección y **nunca ejecuta código**, aunque la dirección sea un contrato.

## Valores de retorno

El `return` de la función llamada vuelve con el tipo que declara la interfaz. En ejecución, el motor comprueba que la función llamada tenga el mismo tipo (`action` o `view`), los mismos parámetros y el mismo retorno; una diferencia falla con `interface mismatch: 'f' is action (text) -> int in the called contract but (int) -> int in the interface`.

## Todo o nada

Una transacción es **atómica**. Si algo falla en cualquier parte — un `require` en un contrato tres llamadas más abajo, un desbordamiento, quedarse sin combustible, una transición no permitida — entonces:

- se descarta todo cambio de almacenamiento en **todos** los contratos involucrados;
- se deshace toda transferencia de TCN, incluidos `with value` y el valor que adjuntó el usuario;
- no se registra ningún evento;
- la comisión de la transacción se paga igual, porque la red la ejecutó.

No hay `try`/`catch` en la versión 2: un contrato no puede tragarse el fallo de otro y seguir en un estado a medio actualizar. Diseñe los flujos para que un fallo sea un "no" claro. Las billeteras simulan las llamadas antes y no envían llamadas que fallarían.

## La reentrada es imposible

Un contrato que ya se está ejecutando en la transacción **no puede volver a ser llamado** durante ella. Si `A` llama a `B` y `B` intenta llamar a `A`, la llamada falla con `re-entrant call` y todo se revierte. Un contrato que se llama a sí mismo mediante una interfaz se rechaza igual. Esta protección forma parte del motor y no se puede desactivar.

Esto elimina el ataque clásico en el que el contrato llamado vuelve a entrar en uno a medio actualizar. Aun así, es buena práctica actualizar su propio estado antes de llamar fuera (*verificaciones → efectos → llamadas*), porque mantiene el código fácil de razonar:

```tccl
action withdraw(token: address, amount: int):
    require deposits[caller] >= amount, "not enough"
    deposits[caller] -= amount                         # primero los efectos
    require Token(token).transfer(caller, amount), "transfer failed"   # después la llamada
```

Por eso no hay callbacks (un token avisando al contrato que lo llamó). Use patrones de consulta: el otro contrato expone una view o una action que usted llama.

## Views y llamadas de solo lectura

- Una `view` puede llamar views de otros contratos. No puede llamar actions — es un error de compilación.
- Todo lo que se alcanza desde una view se ejecuta en solo lectura: un intento de cambiar estado falla con `state cannot be modified in a view`.

## Verificar a quién llama

```tccl
init(token_address: address):
    require is_contract(token_address), "not a contract"
    require is_final(token_address), "the token must not be upgradeable"
    token = token_address
    token_code = code_hash(token_address)
```

- `is_contract(addr)` — si allí existe un contrato.
- `is_final(addr)` — el contrato no tiene autoridad de actualización, así que su código nunca puede cambiar ([Actualizaciones](upgrades.md)).
- `code_hash(addr)` — BLAKE3 de su código compilado; compárelo con el hash del código que revisó.

## Límites y costos

| | |
|---|---|
| Contratos en la pila de llamadas | 8 (`contract call depth limit reached`) |
| Profundidad de llamadas a funciones, sumando todos los contratos | 16 |
| Combustible | Un solo presupuesto para toda la transacción, compartido por todos los contratos |
| Memoria | 16 MiB de valores sumando todas las funciones en ejecución |
| Costo de una llamada | 700 de combustible + 1 cada 32 bytes de argumentos + carga de 100 + 1 cada 100 bytes de código |
| `destroy()` | Solo cuando el contrato es llamado directamente por una transacción |
| Contratos de la versión 1 del lenguaje | No pueden ser llamados por otros contratos (`not supported: calls into language version 1 contracts`): se escribieron cuando `caller` siempre era quien firmaba |

## Ejemplo: un pool de intercambio

La receta [`pool.tccl`](recipes.md#exchange-pool) es un exchange de producto constante que guarda dos tokens. `add_liquidity` y `swap` actualizan las reservas y luego llaman `transfer_from`/`transfer` en los contratos de los tokens. Si un operador no aprobó suficientes tokens, el `require` del token falla y todo el swap — incluida la actualización de reservas — se revierte. Su escenario prueba exactamente eso.

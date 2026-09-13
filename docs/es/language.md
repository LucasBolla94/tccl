# Referencia del lenguaje

Esta página describe la versión 2 del lenguaje. Lo marcado **v2** es nuevo en la versión 2; todo lo demás también existe en la versión 1 y allí se comporta igual.

## Estructura del archivo

```tccl
contract Shop                      # 1. encabezado: la primera línea de código

use std.token                      # 2. módulos (v2)

const MAX_NAME: int = 64           # 3. declaraciones, en cualquier orden
state owner: address
state prices: map[text, int]
event Sold(item: text, buyer: address, price: int)
role clerk                         # v2

init():                            # 4. funciones
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

- La primera línea de código es `contract Nombre`. Un archivo contiene un contrato; después pueden venir secciones `module` ([Módulos](modules.md)).
- Los bloques se definen por la sangría **solo con espacios** (una tabulación es un error de compilación). Las líneas dentro de `( )` y `[ ]` pueden continuar en la siguiente. Los comentarios empiezan con `#`.
- Las declaraciones pueden aparecer en cualquier orden. Las funciones pueden llamar auxiliares declarados después. Una constante solo puede usar constantes declaradas antes.
- Un contrato necesita al menos un `init`, `action` o `view`.
- Un nombre se declara una vez por contrato y no puede ocultar una variable local ni un nombre integrado.

**Palabras clave:** `contract const state event init action view fn payable let if elif else while for in break continue return require send emit destroy pass true false and or not`

**Palabras contextuales (v2):** `module use as record enum interface role only grant revoke to from with value upgrade`. Solo son palabras clave donde la gramática las espera, así que los contratos de la versión 1 que las usan como nombres (por ejemplo un parámetro llamado `to`) siguen compilando.

**Nombres reservados:** `caller value balance height self TCN int bool text bytes address list map len sha256 blake3 to_bytes to_text to_int min max abs slice verify_ed25519 ring_verify address_of zero_address range`. Las funciones de la v2 `mul_div isqrt pow code_hash is_contract is_final` y el valor de contexto `origin` no están reservados: si un contrato declara uno de esos nombres, vale su declaración.

## Tipos

| Tipo | Predeterminado | Literal | Notas |
|---|---|---|---|
| `int` | `0` | `42`, `-7`, `1_000_000` | Entero con signo de 128 bits, aritmética verificada |
| `bool` | `false` | `true`, `false` | |
| `text` | `""` | `"hola\n"` | UTF-8; `len` cuenta bytes; escapes `\n \t \" \\` |
| `bytes` | vacío | `0xdead_beef` | Número par de dígitos hexadecimales |
| `address` | dirección cero | `address("tc1…")` | 20 bytes; el prefijo debe ser el de la red |
| `list[T]` | `[]` | `[1, 2, 3]` | `T` es cualquier tipo salvo map |
| `map[K, V]` | vacío | — | Solo en variables de estado; `K` es escalar o enum |
| record **v2** | todos los campos en su valor predeterminado | `Order(buyer: caller, amount: 5)` | Campos con nombre y tipo |
| enum **v2** | primera variante | `Status.Open` | Situaciones con nombre, transiciones opcionales |
| interface **v2** | dirección cero | `Token(addr)` | Una dirección usada para llamar a otro contrato |

Los escalares (`int bool text bytes address`) se pueden comparar con `==` y `!=`, usar como claves de map y recibir valor inicial en `state`. Los enums también se pueden comparar, usar como claves y recibir valor inicial. Los records y las listas no se pueden comparar con `==`.

No hay conversiones implícitas ni decimales. Los montos son enteros en **motes**: 1 TCN = `TCN` = 100 000 000 motes. En el código escriba `5 * TCN`; la notación `5tcn` es solo para argumentos de línea de comandos y de escenarios.

## Constantes y estado

```tccl
const FEE_BP: int = 25                        # calculada al compilar
const TREASURY: address = address("tc1yfugpgq45fs8xe80x4jam2j2w2kqhk9qnp2umy")
state owner: address                          # predeterminado: dirección cero
state name: text = "Cloud Token"              # escalares y enums pueden tener valor inicial constante
state phase: Phase = Phase.Open               # v2
state holders: list[address]                  # se guarda elemento por elemento
state balances: map[address, int]
state orders: map[int, Order]                 # v2: records en maps
```

Las constantes pueden usar literales, constantes anteriores, operadores, variantes de enum y `address("…")`.

Las listas y maps de estado no se pueden asignar ni copiar enteros: trabaje con sus elementos. **Los valores predeterminados nunca se guardan:** asignar `0`, `false`, `""`, bytes o lista vacíos, la dirección cero, la primera variante de un enum o un record con todos sus campos en el valor predeterminado borra la entrada, así que `m.has(k)` pasa a ser `false` y se reembolsa el depósito de almacenamiento.

## Funciones

| Tipo | Encabezado | Notas |
|---|---|---|
| `init` | `init(params) [payable]:` | Opcional, como máximo uno; se ejecuta al desplegar; no devuelve valor |
| `action` | `action nombre(params) [-> T] [payable] [only …]:` | La llaman transacciones; puede devolver un valor (`return_value` en el recibo) |
| `view` | `view nombre(params) -> T:` | Consulta gratuita de solo lectura; debe devolver un valor |
| `fn` | `fn nombre(params) [-> T] [only …]:` | Auxiliar privado, solo lo llama el código |
| `upgrade` **v2** | `upgrade(params) [only …]:` | Se ejecuta una vez cuando una actualización instala este código ([Actualizaciones](upgrades.md)) |

- La versión 2 acepta `payable` antes o después del tipo de retorno. La versión 1 exige `-> T payable`.
- Solo `action` e `init` pueden ser `payable`. Enviar TCN a cualquier otra función falla con `function does not accept TCN (not payable)`.
- Una view no puede asignar estado, usar `send`, `emit` o `destroy`, leer `value`, llamar a un `fn` que haga algo de eso ni llamar a una action de otro contrato.
- Una función con tipo de retorno debe retornar en todos los caminos: su última instrucción es un `return`, o un `if`/`else` cuyas ramas terminan todas en `return`.
- Los puntos de entrada no se pueden llamar desde el código. La recursión se permite hasta una profundidad de 16.
- `only` enumera roles o variables de estado de tipo address; vea [Permisos](permissions.md).

## Instrucciones

```tccl
let fee: int = amount / 100              # locales: tipo y valor obligatorios
balances[to] += amount - fee             # = += -= *=  (+= también une text y bytes)
orders[id].status = Status.Paid          # v2: campos de record, en locales y en almacenamiento
require caller == owner, "only the owner"
if amount > 100 * TCN:
    pass
elif amount > 0:
    queue.push(to)
else:
    require false, "amount must be positive"
for i in range(0, 3):                    # inicio … fin-1
    continue
for who in queue:                        # lista local o de estado
    break
while len(queue) > 10:
    queue.pop()
send(to, fee)                            # paga con el saldo del contrato
emit Paid(to, fee)
grant clerk to who                       # v2
revoke clerk from who                    # v2
```

- `require cond[, mensaje]` detiene la llamada y **revierte todo efecto** cuando `cond` es falsa (`requirement failed: mensaje`; sin mensaje: `requirement at line N failed`).
- `send(to, amount)` falla con `invalid amount` (≤ 0) o `insufficient contract balance`. Nunca ejecuta código en el destinatario.
- `emit Evento(args…)` — los argumentos deben coincidir con los campos declarados. Los eventos van al recibo.
- `destroy(to)` — solo en actions: borra el contrato y paga su saldo y su depósito a `to`. Antes hay que eliminar todos los elementos de listas y entradas de maps. No se permite cuando otro contrato llamó a este.
- `return [valor]`, `break`, `continue`, `pass`.
- Una expresión sola en una línea debe ser una llamada a función, una llamada a otro contrato o `xs.pop()`.
- El índice de una asignación compuesta se evalúa una vez: `m[next_id()] += 1` llama a `next_id()` una sola vez.

## Operadores

| Precedencia (menor → mayor) | Operadores | Tipos |
|---|---|---|
| 1 | `or` | bool, cortocircuito |
| 2 | `and` | bool, cortocircuito |
| 3 | `not` | bool |
| 4 | `==` `!=` · `<` `<=` `>` `>=` | mismo tipo escalar o enum · int |
| 5 | `+` `-` | int; `+` también text + text, bytes + bytes |
| 6 | `*` `/` `%` | int |
| 7 | `-` unario | int |
| 8 | `x[i]` `x.campo` `x.m(…)` `f(…)` | |

El desbordamiento falla con `integer overflow`. `/` y `%` por cero fallan. `/` trunca hacia cero y `%` toma el signo del operando izquierdo (`-7 / 2 == -3`, `-7 % 2 == -1`). Las comparaciones encadenadas (`a < b < c`) se rechazan. Las condiciones deben ser `bool`: no existe la "veracidad" implícita.

Un valor de interfaz se puede comparar con una dirección: `Token(a) == b`.

## Listas y maps

| Operación | Lista local | Lista de estado |
|---|---|---|
| literal `[a, b]`, `[]` | ✓ | — |
| `xs[i]`, `xs[i] = v`, `xs[i] += v` | ✓ | ✓ |
| `xs.push(v)` | ✓ | ✓ |
| `xs.pop()` (instrucción o expresión) | — | ✓ |
| `len(xs)` / `xs.len()` | ✓ | ✓ |
| `for x in xs:` | ✓ | ✓ |
| asignar o copiar la lista entera | ✓ | — |

Las listas locales tienen como máximo 4 096 elementos y 65 536 bytes. Los maps admiten `m[k]` (el valor predeterminado si falta), `m[k] = v`, `m[k] += v`, `m.has(k)` y `m.remove(k)`. Los maps no se pueden recorrer: guarde una lista de claves si la necesita. Para cambiar una lista guardada en un map, cópiela a una local, modifíquela y vuelva a guardarla.

## Records **v2** {#records}

Un record agrupa campos con nombre y tipo.

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

- Construya un record con **cada campo nombrado exactamente una vez**: `Order(buyer: …, item: …, price: …, status: …)`. Los campos que faltan, desconocidos o repetidos son errores de compilación.
- Lea campos con `.`: `o.price`, `orders[id].buyer`. Asígnelos en locales y en almacenamiento: `o.price = 5`, `orders[id].status = Status.Paid`, `orders[id].price += 1`.
- Un campo puede tener cualquier tipo salvo map, incluido otro record o una lista. Un record no puede contenerse a sí mismo.
- Los records pueden ser parámetros, valores de retorno, campos de eventos, variables de estado y valores de maps y listas. No pueden ser claves de map ni compararse con `==`.
- Un record con todos los campos en su valor predeterminado es el record predeterminado: guardarlo borra la entrada.
- Los records declarados en un módulo se llaman `modulo.Record` desde el contrato.
- En la ABI, los argumentos se escriben `{buyer: tc1…, item: "lamp", price: 5, status: Placed}`.

## Enums y transiciones **v2** {#enums}

Un enum enumera situaciones con nombre. La primera variante es la predeterminada.

```tccl
enum Color: Red, Green, Blue           # forma en línea, sin transiciones

enum Status:                           # forma en bloque con transiciones permitidas
    Placed -> Paid, Cancelled
    Paid -> Shipped, Refunded
    Shipped -> Delivered
    Delivered
    Cancelled
    Refunded
```

- Escriba las variantes como `Status.Paid` (o `modulo.Status.Paid`). `to_text(s)` devuelve el nombre de la variante, por ejemplo `"Paid"`.
- Los enums se pueden comparar con `==`/`!=` y usar como claves de map, constantes, valores iniciales de estado, campos de record y parámetros (los argumentos se escriben con el nombre de la variante: `Paid`).
- Si **alguna** variante enumera `-> siguiente, …`, las transiciones se aplican a ese enum. Una variante sin flecha es final: nada puede seguirla.
- Las transiciones se verifican **cada vez que se guarda un valor**: en una variable de estado, un valor de map, un elemento de lista de estado o un campo de un record guardado en cualquiera de ellos. Cambiar `orders[id].status` de `Placed` a `Delivered` falla con `transition not allowed: Status cannot go from Placed to Delivered`, y se revierte toda la llamada.
- Quedarse en la misma variante siempre está permitido, así que actualizar otros campos de un record no dispara la verificación.
- Una entrada nueva de map empieza en la variante predeterminada (la primera); un `push` en una lista de estado también se verifica desde la predeterminada.
- Las variables locales no se verifican: solo importa lo que se guarda.
- Una actualización puede agregar variantes al final, nunca reordenarlas ni eliminarlas ([Actualizaciones](upgrades.md)).

## Roles y `only` **v2**

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

- `role nombre` declara un conjunto de direcciones que guarda el contrato.
- `grant nombre to <dirección>` y `revoke nombre from <dirección>` lo modifican y emiten automáticamente `RoleGranted(role, account, by)` o `RoleRevoked(role, account, by)`.
- `nombre.has(dirección)` indica si una dirección tiene el rol.
- `only a, b` en una action, `fn` o `upgrade()` solo deja continuar la llamada si `caller` tiene el rol `a`, o es la dirección guardada en `b` (variable de estado de tipo address), etcétera. Si no, falla con `only a or b can call 'f'`.
- Las reglas aparecen en la interfaz (`tccl check`, `tccl abi`, el playground) para que los usuarios vean quién puede hacer qué.

Detalles y patrones: [Permisos](permissions.md).

## Interfaces y llamadas **v2**

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

- `Nombre(dirección)` trata una dirección como un contrato con esa interfaz; `let t: Token = Token(a)` la guarda.
- Dentro del contrato llamado, `caller` es **el contrato que llama**; `origin` es la cuenta que firmó la transacción.
- `with value <monto>` envía TCN desde el saldo del contrato que llama; la interfaz debe marcar la función como `payable`.
- Las firmas de interfaz usan `int bool text bytes address` y listas de ellos. En ejecución, la función llamada debe tener exactamente el mismo tipo, parámetros y retorno; si no, la llamada falla con `interface mismatch`.
- Un fallo en el contrato llamado revierte toda la transacción. Un contrato que ya se está ejecutando no puede volver a ser llamado (la reentrada siempre se rechaza).

Todo sobre llamadas — identidad, transferencias, atomicidad, límites — está en [Llamar a otros contratos](calls.md).

## Valores de contexto

| Nombre | Tipo | Significado |
|---|---|---|
| `caller` | `address` | Quién llamó a esta función: quien firmó o el contrato que llama. Dirección cero dentro de views |
| `origin` **v2** | `address` | La cuenta que firmó la transacción, incluso dentro de una llamada desde otro contrato |
| `value` | `int` | Motes enviados en esta llamada (no disponible en views) |
| `balance` | `int` | Saldo de este contrato en motes, incluido `value` |
| `height` | `int` | Altura del bloque (aproximadamente un bloque por minuto) |
| `self` | `address` | La dirección de este contrato |
| `TCN` | `int` | 100 000 000 |

## Funciones integradas

| Función | Resultado | Combustible extra |
|---|---|---|
| `len(list \| text \| bytes)` | `int` | — |
| `min(a, b)`, `max(a, b)`, `abs(a)` | `int` | — |
| `to_text(int \| enum)` | texto decimal o nombre de la variante | — |
| `to_bytes(int \| address \| text \| bool \| bytes)` | `bytes` (int: 16 bytes big-endian; address: 20; bool: 1) | — |
| `to_int(bytes)` | `int` sin signo big-endian de hasta 15 bytes | — |
| `slice(bytes, start, end)` | `bytes` | — |
| `sha256(bytes \| text)`, `blake3(bytes \| text)` | 32 bytes | 60 + 20 cada 64 bytes |
| `verify_ed25519(pk, msg, sig)` | `bool` (false si la entrada está mal formada) | 3 500 + 1 cada 64 bytes |
| `ring_verify(ring: list[bytes], msg, sig, key_image)` | `bool` — firma en anillo vinculable (bLSAG, Ristretto255), ≤ 64 claves | 5 000 + 10 000 por clave |
| `address_of(pk: bytes)` | `address` de una clave pública de 32 bytes | — |
| `zero_address()` | `address` | — |
| `address("tc1…")` | dirección literal en tiempo de compilación | — |
| `mul_div(a, b, c)` **v2** | ⌊a × b ÷ c⌋ hacia cero, sin desbordamiento intermedio | 30 |
| `isqrt(x)` **v2** | ⌊√x⌋ para x ≥ 0 | 30 |
| `pow(base, exp)` **v2** | potencia entera verificada, exp ≥ 0 | 30 |
| `code_hash(addr)` **v2** | BLAKE3 del código compilado de un contrato, vacío si no es contrato | 250 |
| `is_contract(addr)` **v2** | `bool` | 250 |
| `is_final(addr)` **v2** | `true` si el contrato no tiene autoridad de actualización | 250 |

```tccl
let msg: bytes = blake3(to_bytes(self) + to_bytes(caller) + to_bytes(amount))
require verify_ed25519(pk, msg, sig), "bad signature"
let out: int = mul_div(amount_in * 9_970, reserve_out, reserve_in * 10_000 + amount_in * 9_970)
```

## Límites

| Límite | Valor |
|---|---|
| Código fuente (un archivo, incluidas las secciones de módulo) | 48 000 bytes |
| Programa compilado | 262 144 bytes |
| Combustible por transacción · por bloque (predeterminado de mainnet) | 10 000 000 · 50 000 000 |
| Profundidad de llamadas a funciones | 16 |
| Contratos en la pila de llamadas **v2** | 8 |
| Memoria usada por las funciones en ejecución **v2** | 16 MiB por transacción |
| Anidamiento de bloques, paréntesis, tipos | 32 |
| Operadores de una misma precedencia en una expresión · profundidad de expresión | 64 · 128 |
| Funciones · variables de estado · locales por función | 256 · 256 · 1 024 |
| Records · campos por record · enums · variantes · interfaces **v2** | 128 · 64 · 128 · 256 · 64 |
| Módulos por contrato **v2** | 16 |
| Valor text, bytes o lista | 65 536 bytes |
| Elementos en una lista local | 4 096 |
| Eventos por llamada · argumentos por llamada | 64 · 32 |
| Tamaño del anillo | 64 |

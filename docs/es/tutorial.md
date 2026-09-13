# Su primer contrato

Este tutorial dura unos diez minutos. Instalará `tccl`, creará un contrato, encontrará y corregirá un error, lo probará con un escenario, simulará transacciones y aprenderá a desplegarlo en The Coin.

> Todo lo de los pasos 1 a 6 ocurre en su computadora. No se firma ni se publica nada, y no se usa ninguna billetera ni frase de recuperación.

## 1. Instalar

**Linux**

```sh
curl -fsSL https://tccl.the-coin.cloud/install.sh | sh
```

**Windows** (PowerShell)

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://tccl.the-coin.cloud/install.ps1 | iex"
```

Los dos scripts descargan la última versión desde GitHub y verifican su checksum SHA-256. También puede usar el instalador MSI o compilar desde el código — vea [Herramientas](tools.md#install). Verifique la instalación:

```sh
tccl version
```

¿Prefiere no instalar nada? Abra el [playground](/es/playground/): ejecuta el mismo compilador y la misma máquina virtual en su navegador.

## 2. Crear un proyecto

```sh
tccl new tip-counter
cd tip-counter
```

Esto crea dos archivos: `contract.tccl` y `contract.scenario`. Abra `contract.tccl`:

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

Léalo de arriba abajo:

- Las líneas que empiezan con `#` son comentarios.
- `contract TipCounter` da nombre al contrato. Un archivo contiene un contrato.
- Las variables `state` se guardan en la cadena. `owner` empieza como la dirección cero y `count` como `0`.
- `event Increased(...)` declara algo que el contrato puede anunciar. Los eventos van al recibo de la transacción, donde las billeteras y los exploradores los leen.
- `init()` se ejecuta una vez, al desplegar. `caller` es quien envió la transacción, así que quien despliega pasa a ser el dueño.
- Una `action` la llama una transacción y puede cambiar el estado. `require` detiene la llamada — y revierte todo — cuando su condición es falsa.
- `only owner` deja que solo la dirección guardada en `owner` llame a `reset`.
- Una `view` es una consulta gratuita de solo lectura. Debe devolver un valor.

Las palabras clave y los mensajes del contrato quedan en inglés, como en cualquier lenguaje de programación; los nombres y textos son suyos.

## 3. Verificar

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

`tccl check` compila exactamente como la red y muestra la interfaz: cada punto de entrada, quién puede llamarlo y qué puede hacer el contrato.

## 4. Romperlo a propósito

Cambie `return count` por `return cout` y verifique de nuevo, con explicaciones en español:

```sh
tccl check contract.tccl --lang es
```

```text
error[C006]: unknown name 'cout'
  --> contract.tccl:21:12
   |
21 |     return cout
   |            ^
  = por qué: Todo nombre debe declararse: un 'let' local, un parámetro, una const, una variable de estado o un valor de contexto (caller, value, balance, height, self, origin).
  = corrección: Revise la ortografía o declárelo, p. ej. 'let total: int = 0'. (did you mean 'count'?)
```

Cada error tiene una posición, un código que puede consultar con `tccl explain C006 --lang es` o en la [referencia de errores](errors.md), una explicación y una corrección. El mensaje en sí queda en inglés porque forma parte del comportamiento del compilador. Deshaga el cambio antes de continuar.

## 5. Probarlo con un escenario

`contract.scenario` es un pequeño guion de prueba:

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

Ejecute todos los escenarios de la carpeta:

```sh
tccl test
```

```text
✔ ./contract.scenario (5 checks)
5 checks passed, 0 failed, 1 scenario file(s)
```

Cuentas como `owner` y `bob` son ficticias; cada una empieza con 1 000 000 TCN. Los escenarios son la forma más rápida de demostrar que los permisos y los fallos se comportan como usted quiere. La sintaxis completa está en [Herramientas](tools.md#scenarios).

## 6. Simular transacciones

`tccl run` mantiene una cadena simulada en `tccl-state.json` para explorar paso a paso:

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
```

El simulador muestra el combustible usado y una **estimación** de la comisión y del depósito de almacenamiento con los parámetros predeterminados de The Coin. Las direcciones empiezan con `tcr1` porque las cuentas simuladas no son cuentas de mainnet (`tc1`) ni de testnet (`tct1`).

## 7. Desplegar en The Coin

El despliegue usa la billetera del software del nodo, [`thecoin-wallet`](https://the-coin.cloud/docs.html), no `tccl`. `tccl` nunca maneja claves.

> **Versión del lenguaje en la red.** The Coin v0.2.0 ejecuta la versión 1 del lenguaje. La plantilla de arriba usa `only`, que es de la versión 2. Para desplegar hoy, reemplace `only owner` por `require caller == owner, "only the owner"` y verifique con `tccl check --language 1 contract.tccl`.

Empiece siempre en testnet:

```sh
thecoin-wallet --network testnet contract deploy contract.tccl
thecoin-wallet --network testnet contract invoke <dirección> increment 5
thecoin-wallet --network testnet contract view <dirección> get
```

La billetera simula cada llamada antes de enviarla y se niega a enviar llamadas que fallarían. Vea [Herramientas](tools.md#testnet) y [Combustible, comisiones y depósitos](fees.md).

## Próximos pasos

- Modifique los ejemplos del [playground](/es/playground/) — pruebe `orders.tccl` y `pool.tccl`.
- Lea la [referencia del lenguaje](language.md).
- Antes de manejar valor real, lea [Seguridad](security.md) y escriba un escenario para cada `require`.

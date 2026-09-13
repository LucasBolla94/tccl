# Referencia de errores

> Esta página se genera a partir del catálogo del compilador (`tccl docs errors`). Las mismas explicaciones aparecen en `tccl check`, `tccl explain <código>` y el playground.

Los *mensajes* de error siempre están en inglés porque forman parte del comportamiento del compilador (y, en la versión 1 del lenguaje, del historial de la cadena). El código, la explicación y la corrección están traducidos.

## Errores de compilación

Un error de compilación se muestra como `archivo:línea:columna`, con el código, una corrección sugerida y a menudo una pista “¿quiso decir?”. Un contrato que no compila nunca se despliega; en la red, una transacción de despliegue cuyo código no compila es rechazada por la billetera antes de enviarla, y paga la comisión si aun así se mina.

### C001 — Carácter de tabulación

Los bloques se definen por la sangría y TCCL solo acepta espacios, para que el significado de una línea no dependa del editor.

**Corrección:** Reemplace las tabulaciones por 4 espacios.

Mensajes incluyen: `tabs are not allowed`

### C002 — Sangría

Después de una línea que termina en ':' las siguientes deben tener más sangría; el bloque termina cuando la sangría vuelve a un nivel exterior.

**Corrección:** Sangre el cuerpo de cada bloque con el mismo número de espacios (normalmente 4); use 'pass' para un bloque vacío.

Mensajes incluyen: `indentation does not match`, `unexpected indentation`, `an indented block`, `empty block`

### C003 — Texto sin terminar

Un texto debe empezar y terminar con " en la misma línea. Escapes: \n \t \" \\.

**Corrección:** Cierre las comillas o escape las internas como \".

Mensajes incluyen: `unterminated`, `unknown escape`

### C004 — Literal inválido

Los enteros son decimales (con _ opcional) y caben en 128 bits; los bytes son 0x seguido de un número par de dígitos hexadecimales. No hay decimales: los montos son enteros en motes.

**Corrección:** Escriba montos como enteros, p. ej. 5 * TCN o 250_000_000.

Mensajes incluyen: `invalid number literal`, `integer literal out of range`, `hex bytes literal`, `invalid hex literal`

### C005 — Estructura del contrato

Un archivo desplegable empieza con 'contract Nombre' y declara al menos un init, action o view. Los módulos pueden seguir como secciones 'module nombre'.

**Corrección:** Ponga 'contract MiContrato' en la primera línea de código.

Mensajes incluyen: `'contract <Name>'`, `only one contract`, `a contract needs at least one`, `only contains modules`

### C006 — Nombre desconocido

Todo nombre debe declararse: un 'let' local, un parámetro, una const, una variable de estado o un valor de contexto (caller, value, balance, height, self, origin).

**Corrección:** Revise la ortografía o declárelo, p. ej. 'let total: int = 0'.

Mensajes incluyen: `unknown name`, `unknown variable`

### C007 — Tipo desconocido

Los tipos son int, bool, text, bytes, address, list[T], map[K, V] (solo en estado) y los records, enums e interfaces que declare.

**Corrección:** Use un tipo integrado o declare el record/enum.

Mensajes incluyen: `unknown type`

### C008 — Llamada a función

Las llamadas deben usar una función integrada o un 'fn' con la cantidad y los tipos correctos de argumentos.

**Corrección:** Revise el nombre y los parámetros en la declaración.

Mensajes incluyen: `unknown function`, `takes 0 argument`, `takes 1 argument`, `takes 2 argument`, `takes 3 argument`, `argument(s),`

### C009 — Tipos incompatibles

TCCL nunca convierte tipos implícitamente: una operación o asignación necesita exactamente el tipo declarado. Las condiciones deben ser bool.

**Corrección:** Convierta explícitamente con to_text, to_bytes o to_int, o cambie el tipo declarado.

Mensajes incluyen: `expected int, found`, `expected bool, found`, `expected text, found`, `expected bytes, found`, `expected address, found`, `cannot add`, `arithmetic needs`, `cannot compare`, `ordering comparisons`, `need bool operands`, `compound assignment is not defined`, `expected a `

### C010 — Falta return

Una función que declara '-> tipo' debe terminar con 'return', o con un if/else donde cada rama retorna.

**Corrección:** Agregue un 'return <valor>' final.

Mensajes incluyen: `must return a`, `must return on every path`, `does not return a value`

### C011 — Las views solo leen

Una view es una consulta gratuita que cualquiera ejecuta sin transacción, así que no puede cambiar estado, enviar TCN, emitir eventos, leer 'value' ni llamar actions.

**Corrección:** Conviértala en 'action' o mueva el cambio a una action.

Mensajes incluyen: `a view cannot`, `changes state, sends TCN or emits events`, `not available in a view`, `view cannot call the action`

### C012 — Nombre ya usado

Cada nombre se declara una vez por contrato y no puede ocultar nombres integrados (caller, value, len, ...).

**Corrección:** Elija otro nombre.

Mensajes incluyen: `already declared`, `is reserved`, `reserved name`, `duplicate`

### C013 — Maps y listas de estado

Los maps solo existen como variables de estado y se usan elemento por elemento; listas y maps de estado no se copian ni asignan enteros.

**Corrección:** Use m[clave], m.has(clave), m.remove(clave), xs[i], xs.push(v), len(xs).

Mensajes incluyen: `maps can only be used as state`, `map keys must be`, `cannot assign a whole`, `is a map;`, `is a state list`

### C014 — Constantes

Las constantes se calculan al compilar a partir de literales, otras constantes, aritmética, variantes de enum y address("..."), y nunca cambian.

**Corrección:** Use una variable de estado para valores que cambian.

Mensajes incluyen: `is a constant`, `is not a constant`, `constant values can only`, `constant expression`, `constants must be`

### C015 — Recibir TCN

Solo las funciones marcadas 'payable' (actions e init) reciben TCN; una llamada a otro contrato solo puede adjuntar TCN con 'with value' si la interfaz la marca payable.

**Corrección:** Agregue 'payable' al encabezado: action buy() payable:

Mensajes incluyen: `can be payable`, `not payable in its interface`, `'with value'`, `views cannot receive TCN`

### C016 — Los entry points no son auxiliares

init, actions y views los llaman transacciones y consultas; el código solo llama auxiliares 'fn' (y otros contratos mediante interfaces).

**Corrección:** Mueva la lógica común a un 'fn' y llámelo desde ambos lugares.

Mensajes incluyen: `is an entry point`

### C017 — Instrucción

Una línea debe hacer algo: asignar, llamar una función, controlar el flujo. Las comparaciones no se encadenan (use 'and').

**Corrección:** Guarde el resultado con 'let' o elimine la línea.

Mensajes incluyen: `does nothing as a statement`, `outside of a loop`, `range() can only`, `chained comparisons`

### C018 — Records

Un record agrupa campos con nombre y tipo. Constrúyalo nombrando todos los campos, p. ej. Order(buyer: caller, amount: 5); léalo con order.amount.

**Corrección:** Indique cada campo una vez con 'nombre: valor'.

Mensajes incluyen: `record`, `has no field`, `needs every field`, `named fields`, `contains itself`

### C019 — Enums y transiciones

Un enum enumera situaciones con nombre. Líneas como 'Open -> Paid, Cancelled' declaran los cambios permitidos; cualquier otro falla al guardar.

**Corrección:** Use Enum.Variante y liste las siguientes permitidas tras '->'.

Mensajes incluyen: `variant`, `'->'`

### C020 — Roles y permisos

'only' restringe quién puede llamar una action: miembros de un rol (role admin; grant admin to x) o la dirección de una variable de estado.

**Corrección:** Declare 'role nombre' y concédalo en init(): grant nombre to caller.

Mensajes incluyen: `'only`, `unknown role`, `is a role`

### C021 — Interfaces

Una interfaz describe actions y views de otro contrato. Las llamadas la usan: Token(addr).transfer(to, 5). Las firmas usan int, bool, text, bytes, address y listas.

**Corrección:** Declare la función en la interfaz exactamente como la define el otro contrato.

Mensajes incluyen: `interface`

### C022 — Módulos

Un módulo es código reutilizable compilado dentro del contrato ('use std.token' o una sección 'module nombre'). No tiene dirección; su estado solo cambia con sus propias funciones.

**Corrección:** Llame las funciones del módulo: token.mint(to, amount).

Mensajes incluyen: `module`, `std.`

### C023 — Actualizaciones

Una actualización conserva el almacenamiento: las variables de estado no se eliminan ni cambian de tipo, los records conservan sus campos y los enums sus variantes en orden.

**Corrección:** Conserve las variables antiguas (puede dejar de usarlas) y agregue nuevas.

Mensajes incluyen: `upgrade`, `was removed`, `storage slot`

### C024 — Límite alcanzado

El compilador limita tamaño, anidamiento y cantidades para que todo nodo compile cualquier contrato de forma rápida y segura.

**Corrección:** Divida expresiones largas con 'let' y contratos grandes en módulos o varios contratos.

Mensajes incluyen: `too many`, `too large`, `too deep`, `too deeply`, `too long`, `nested too`

### C025 — Dirección literal

address("...") recibe una dirección bech32m válida de la red para la que se compila (tc1 mainnet, tct1 testnet, tcr1 regtest).

**Corrección:** Copie de nuevo la dirección y revise el prefijo.

Mensajes incluyen: `address literal`, `address prefix`, `invalid address`

### C026 — Sintaxis

La línea no sigue la gramática en la posición marcada.

**Corrección:** Compare con la referencia del lenguaje o un ejemplo.

Mensajes incluyen: `expected`

## Errores de ejecución

Un error de ejecución detiene la llamada. **Se revierte todo efecto de la transacción** — el almacenamiento de todos los contratos involucrados, el TCN enviado, los eventos y el valor adjunto a la llamada. La comisión de la transacción se paga igual, porque la red hizo el trabajo. Las billeteras simulan las llamadas antes y se niegan a enviar una llamada que fallaría.

| Código | Error | Significado | Corrección |
|---|---|---|---|
| R001 | `out of fuel` | La llamada usó todo el combustible reservado. Todo se revierte; la comisión se paga igual. | Limite bucles, evite recorrer listas que otros pueden agrandar, o reserve más combustible. |
| R002 | `requirement failed: …` | Un 'require' fue falso; la llamada se detiene y todo efecto se revierte. | Lea el mensaje: indica qué condición no se cumplió. |
| R003 | `integer overflow` | Desbordamiento o división por cero; la aritmética de TCCL siempre se verifica. | Valide entradas con require antes de dividir; use mul_div para a × b ÷ c. |
| R004 | `division by zero` | Desbordamiento o división por cero; la aritmética de TCCL siempre se verifica. | Valide entradas con require antes de dividir; use mul_div para a × b ÷ c. |
| R005 | `index I out of bounds (length N)` | Un índice fue negativo o ≥ la longitud (también pop en lista vacía). | require i >= 0 and i < len(xs) |
| R006 | `value too large` | Un valor superó 64 KiB, una lista 4 096 elementos, el límite de eventos, o las funciones en ejecución usaron más de 16 MiB. | Guarde datos en maps de estado en lugar de listas locales grandes. |
| R007 | `call depth limit reached` | Más de 16 llamadas de función anidadas u 8 contratos en la pila. | Cambie la recursión por bucles; reduzca cadenas de llamadas entre contratos. |
| R008 | `unknown function 'f'` | La función no existe, no es del tipo llamado (action/view) o recibió argumentos incorrectos. | Revise la interfaz del contrato (tccl abi). |
| R009 | `function 'f' cannot be called this way` | La función no existe, no es del tipo llamado (action/view) o recibió argumentos incorrectos. | Revise la interfaz del contrato (tccl abi). |
| R010 | `wrong arguments: …` | La función no existe, no es del tipo llamado (action/view) o recibió argumentos incorrectos. | Revise la interfaz del contrato (tccl abi). |
| R011 | `function does not accept TCN (not payable)` | Se envió TCN a una función que no es 'payable'. El valor vuelve con la reversión. | No envíe valor, o marque la action como payable. |
| R012 | `state cannot be modified in a view` | Una view (o un contrato llamado desde una view) intentó cambiar algo. | Use una action. |
| R013 | `invalid amount` | send() necesita un monto positivo y saldo suficiente del contrato. | Registre en el estado lo que el contrato debe y verifíquelo antes de enviar. |
| R014 | `insufficient contract balance` | send() necesita un monto positivo y saldo suficiente del contrato. | Registre en el estado lo que el contrato debe y verifíquelo antes de enviar. |
| R015 | `contract cannot be destroyed while it still has storage (N entries)` | destroy() exige maps y listas vacíos y no se permite cuando otro contrato llamó a este. | Elimine todos los elementos antes y llame destroy directamente desde una transacción. |
| R016 | `host error: …` | La llamada falló y todo cambio se revirtió. | Lea el mensaje para más detalles. |
| R017 | `internal type error: …` | La llamada falló y todo cambio se revirtió. | Lea el mensaje para más detalles. |
| R018 | `re-entrant call: contract … is already running in this transaction` | Se volvió a llamar un contrato que ya se ejecutaba en esta transacción. La red siempre lo bloquea para evitar ataques de reentrada. | Diseñe flujos donde las llamadas vayan en un sentido (A → B) y pase datos por argumentos o retornos. |
| R019 | `contract call depth limit reached` | Más de 16 llamadas de función anidadas u 8 contratos en la pila. | Cambie la recursión por bucles; reduzca cadenas de llamadas entre contratos. |
| R020 | `no contract at …` | No hay contrato en la dirección, o su función no coincide con la interfaz. | Revise la dirección y declare la interfaz igual a la ABI del destino. |
| R021 | `interface mismatch: …` | No hay contrato en la dirección, o su función no coincide con la interfaz. | Revise la dirección y declare la interfaz igual a la ABI del destino. |
| R022 | `memory limit reached (16777216 bytes)` | Un valor superó 64 KiB, una lista 4 096 elementos, el límite de eventos, o las funciones en ejecución usaron más de 16 MiB. | Guarde datos en maps de estado en lugar de listas locales grandes. |
| R023 | `transition not allowed: E cannot go from A to B` | El enum declara qué cambios se permiten y este no está en la lista. | Siga el camino declarado (p. ej. Paid → Shipped → Delivered) o agregue la transición. |
| R024 | `destroy() is only allowed when the contract is called directly by a transaction` | destroy() exige maps y listas vacíos y no se permite cuando otro contrato llamó a este. | Elimine todos los elementos antes y llame destroy directamente desde una transacción. |
| R025 | `not supported: …` | La llamada falló y todo cambio se revirtió. | Lea el mensaje para más detalles. |

# Seguridad

Esta página explica qué garantiza el motor, qué no puede garantizar y cómo escribir contratos que sigan siendo seguros.

## Garantías del motor

Son propiedades del compilador y de la máquina virtual TCCL. Ningún contrato, opción o configuración las desactiva.

| Garantía | Cómo |
|---|---|
| **Determinismo** | Sin reloj, coma flotante, aleatoriedad, hilos ni iteración desordenada. La misma llamada sobre el mismo estado da el mismo resultado, combustible y almacenamiento en todos los nodos. El compilador forma parte del consenso: cada nodo compila por sí mismo el código desplegado. |
| **Aritmética verificada** | Toda operación con `int` se verifica. El desbordamiento y la división por cero hacen fallar la llamada; `mul_div` evita el desbordamiento intermedio. |
| **Combustible** | Cada instrucción, expresión, acceso al almacenamiento, hash, firma y llamada cuesta combustible. Cuando se agota el combustible de la transacción, la llamada falla. Un bloque tiene un límite de combustible, así que su tiempo de ejecución está acotado. |
| **Límites de memoria** | Los valores tienen como máximo 64 KiB, las listas locales 4 096 elementos y, en la versión 2, todas las funciones en ejecución juntas guardan como máximo 16 MiB. |
| **Compilación acotada** | El tamaño del código, el anidamiento, la profundidad de las expresiones y las cantidades están limitados, así que un código hostil no agota un nodo al compilar. |
| **Tipos** | Cada valor tiene un tipo; las views no cambian nada; solo las funciones payable reciben TCN; los maps solo existen en almacenamiento. |
| **Transacciones atómicas** | Cualquier fallo revierte cada cambio en todos los contratos involucrados, cada transferencia y cada evento. |
| **Sin reentrada** (v2) | Un contrato que se ejecuta en una transacción no puede volver a ser llamado durante ella. |
| **Permisos explícitos** (v2) | Las verificaciones `only` se ejecutan antes del cuerpo de la función y figuran en la interfaz. |
| **Transiciones** (v2) | Las transiciones de enums se verifican en cada escritura. |

## Fallos y comisiones

Cuando una llamada falla — `require`, desbordamiento, falta de combustible, una transición no permitida, un error en otro contrato — **todo lo que hizo la transacción se revierte y la comisión se paga igual**. La red hizo el trabajo, y cobrarlo es lo que impide inundarla con llamadas que fallan.

- Las billeteras simulan cada llamada sobre el estado actual y se niegan a enviar una que fallaría, así que un fallo después del envío suele significar que el estado cambió entre la simulación y el bloque (por ejemplo, otra persona compró el último artículo).
- Usted paga el combustible que **reserva** (`max_fuel`), no solo el usado. La billetera reserva el combustible medido × 1,3 + 5 000.
- Una llamada fallida devuelve el valor adjunto y no cambia ningún depósito.

Detalles y cifras: [Combustible, comisiones y depósitos](fees.md).

## Quién llama, origin y autorización

- Autorice con `caller` u `only`. En una llamada desde otro contrato, `caller` es ese contrato.
- Nunca autorice con `origin`: es quien firmó aunque haya un contrato desconocido en medio.
- Fije dueños y roles en `init()`; no deje una action de configuración sin proteger.

Vea [Permisos](permissions.md) y [Llamar a otros contratos](calls.md).

## Dinero

- Pague a partir de **su propia contabilidad**, no de `balance`: cualquiera puede aumentar el saldo de un contrato con una transferencia simple.
- Actualice el estado antes de enviar o llamar (*verificaciones → efectos → llamadas*).
- Liquide una sola vez: marque el pago como hecho (un estado enum con transiciones es ideal) antes de pagar.
- Prefiera **pagos por retiro** — los usuarios retiran lo que se les debe — antes que recorrer una lista de destinatarios.
- Multiplique antes de dividir, o use `mul_div(a, b, c)`. Rechace montos negativos.

## Denegación de servicio

- Nunca recorra una lista que otras personas pueden hacer crecer sin límite: cada vuelta cuesta combustible y la llamada acabará fallando siempre.
- Guarde los datos por usuario en maps; recorra solo listas acotadas.
- Recuerde que el almacenamiento cuesta un depósito reembolsable que paga quien hace crecer el contrato.

## Actualizaciones

Un contrato actualizable solo es tan confiable como su **autoridad de actualización**, que puede reemplazar el código — incluidas las verificaciones de permisos — de inmediato. Compruebe `is_final(addr)` antes de depender de otro contrato y haga el suyo final cuando sea estable. Vea [Actualizaciones](upgrades.md).

## Privacidad y sus límites {#privacy}

Todo lo guardado en una cadena pública — estado, argumentos, eventos, saldos — es visible para todos, para siempre. `only` controla quién puede *cambiar* los datos, no quién puede *leerlos*.

TCCL ofrece **firmas en anillo vinculables** (`ring_verify`, bLSAG sobre Ristretto255, una construcción consolidada que también usa Monero) para pools de pagos privados como [`private_pool.tccl`](recipes.md#more-recipes). Un retiro demuestra "soy dueño de uno de estos N depósitos" sin decir cuál, y la key image impide retirar dos veces el mismo depósito. Conozca los límites:

- **Conjunto de anonimato.** Usted se oculta entre los miembros del anillo (como máximo 64). Pocos depósitos, o anillos elegidos sin cuidado, revelan mucho.
- **Montos.** Los pools usan una denominación fija; cualquier otro monto llama la atención.
- **Tiempo y comportamiento.** Depositar y retirar seguidos, o con patrones inusuales, lo vincula.
- **Comisiones.** Quien paga la comisión del retiro queda a la vista; use un relayer, en quien debe confiar que no lo registrará.
- **Metadatos.** Su nodo, su dirección IP y el comportamiento de su billetera están fuera del control del contrato.
- **No es conocimiento cero.** Las firmas en anillo ocultan *cuál* miembro firmó, no *que* uno de ellos firmó. No ocultan montos ni la lógica del programa.
- **Nada de criptografía nueva.** TCCL no inventa esquemas criptográficos. No construya el suyo con `sha256` y `blake3`; pida una primitiva revisada.

## Datos externos (oráculos) {#external-data}

Un contrato no puede leer internet. Los precios, resultados y eventos del mundo real solo entran mediante transacciones, así que son exactamente tan confiables como quien los aporta.

- Acepte datos externos **firmados** por claves conocidas (`verify_ed25519`) y vincule el mensaje a `self`, al valor y a una altura o nonce para que no pueda reutilizarse en otro lugar.
- Prefiera varios firmantes independientes y un umbral, y rechace datos viejos usando `height`.
- Planifique qué ocurre cuando el proveedor se detiene o miente: plazos, un árbitro, reembolsos.
- Un secreto revelado en una transacción (por ejemplo para desbloquear un pago bloqueado por hash) es público en cuanto la transacción entra en el mempool.

## Aleatoriedad {#randomness}

**Nada en una blockchain es aleatorio.** La altura del bloque, los hashes de datos anteriores y el contenido de las transacciones se conocen de antemano o pueden ser influidos por mineros y usuarios. Un sorteo que los use puede predecirse o manipularse.

Use **commit–reveal** entre las partes que tienen algo en juego: cada una se compromete con `sha256(secreto + elección)` y luego revela; el contrato verifica el compromiso y combina los secretos. Quien revela último puede negarse a revelar si fuera a perder, así que dele un plazo y haga que negarse le cueste su apuesta. La receta [`coin_flip.tccl`](recipes.md#coin-flip-game) hace exactamente eso. Para muchos participantes, use un umbral de firmantes independientes o una fuente aleatoria verificable firmada fuera de la cadena, y documente la suposición de confianza.

## Front-running

Las transacciones esperan en un mempool público antes de minarse. Cualquiera puede ver una operación pendiente e intentar actuar antes.

- Proteja las operaciones con límites como `min_out` (vea [`pool.tccl`](recipes.md#exchange-pool)).
- Vincule las firmas a `self`, al destinatario, a los montos y a las comisiones.
- Use commit–reveal en subastas y juegos.

## Problemas conocidos en la versión 1 del lenguaje {#v1-findings}

Al construir la versión 2 medimos el motor en uso en The Coin v0.2.0 y encontramos que la versión 1 cobra **las copias de valores por bytes, no por asignaciones de memoria**. Un contrato con una lista local de 4 000 textos cortos que llama a `len(xs)` en un bucle copia la lista entera en cada vuelta por unas 500 unidades de combustible. En la máquina de referencia (Intel Xeon E5-1620 v2) esto cuesta unos **1 400–2 100 ns por unidad de combustible en lugar de ~20**, así que un bloque lleno de esas llamadas podría tardar **uno o dos minutos** en ejecutarse en lugar de un segundo. Leer una lista grande guardada en un map tiene el mismo problema (~1 650 ns por unidad), y una sola transacción puede retener unos 65 MB de valores.

- **La versión 2 lo corrige**: las copias y los valores leídos del almacenamiento pagan por asignación, `xs[i]` y `len(xs)` ya no copian la lista y la memoria se limita a 16 MiB. Las mismas cargas miden 12–21 ns por unidad.
- **La versión 1 no se puede cambiar** sin un cambio de consenso, porque los bloques pasados deben volver a ejecutarse de forma idéntica. Las opciones de mitigación — activar la versión 2 para nuevos despliegues y reajustar el precio del combustible de las llamadas de la versión 1 a partir de una altura de activación — se analizan en [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

Las mediciones se reproducen con `cargo test -p tccl --release --test perf -- --ignored --nocapture`.

## Lista de control antes de manejar valor real {#checklist}

1. Cada action que mueve dinero o cambia permisos tiene `only` o un `require` sobre `caller`.
2. Los dueños y roles se fijan en `init()`.
3. Cada `require` tiene un escenario que lo hace fallar, y cada camino de éxito uno que pasa.
4. Los pagos se liquidan una vez; el estado se actualiza antes de enviar o llamar.
5. Ningún bucle sin límite sobre listas que otros pueden hacer crecer.
6. Montos en motes; multiplicaciones antes que divisiones.
7. Los mensajes firmados incluyen `self`, montos, destinatarios y una altura o nonce.
8. La aleatoriedad usa commit–reveal con plazos; los datos externos van firmados y se comprueba su antigüedad.
9. Las dependencias se verifican con `is_final` o `code_hash`.
10. El contrato se ejecutó en testnet, el código está publicado y alguien distinto del autor lo revisó.
11. La autoridad de actualización es una multifirma o un contrato de gobernanza — o el contrato es final.

## Informar una vulnerabilidad

Por favor, no abra un issue público para una vulnerabilidad en el compilador, la máquina virtual o la biblioteca estándar. Siga [SECURITY.md](https://github.com/LucasBolla94/tccl/blob/main/SECURITY.md).

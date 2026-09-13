# Combustible, comisiones y depósitos

## Combustible

Cada operación cuesta **combustible**. Una transacción reserva un máximo (`max_fuel`); si la ejecución necesita más, la llamada falla y todo se revierte. Los precios están calibrados para que unos 20 ns de CPU en un servidor de 2 vCPU correspondan a una unidad de combustible, lo que mantiene un bloque lleno (50 000 000 de combustible en mainnet de forma predeterminada) en alrededor de un segundo en el peor caso.

| Operación | Combustible |
|---|---|
| Instrucción · expresión | 2 · 1 |
| Cada 32 bytes de valores leídos, copiados o combinados | 1 |
| Llamada a función | 20 |
| Lectura de almacenamiento | 250 |
| Escritura o borrado en almacenamiento | 400 + 4 por byte de clave y valor |
| `send` · `emit` · `destroy` | 300 · 100 + 1 por byte · 1 000 |
| `sha256`, `blake3` | 60 + 20 cada 64 bytes |
| `verify_ed25519` | 3 500 + 1 cada 64 bytes |
| `ring_verify` | 5 000 + 10 000 por clave |
| Despliegue: compilar | 5 por byte de código fuente |
| Llamada: cargar el contrato | 100 + 1 cada 100 bytes de código compilado |

### Cambios en la versión 2 del lenguaje

| Operación | Versión 1 | Versión 2 |
|---|---|---|
| Copiar un valor (leer una local, una constante) | 1 cada 32 bytes | 1 cada 32 bytes **+ 4 por asignación** (cada text, bytes o lista que contiene) |
| Decodificar un valor leído del almacenamiento | incluido en la lectura | **+ 1 cada 32 bytes + 6 por asignación** |
| `xs[i]` y `len(xs)` sobre una lista local | copia la lista entera | lee sin copiar |
| Llamada a otro contrato | — | 700 + 1 cada 32 bytes de argumentos + carga (100 + 1 cada 100 bytes de código) |
| `mul_div`, `isqrt`, `pow` | — | 30 |
| `code_hash`, `is_contract`, `is_final` | — | 250 |
| Verificación de transición de enum | — | 1 por nivel verificado |

El código de la versión 1 compilado como versión 2 produce los mismos resultados, almacenamiento y eventos; su combustible solo puede cambiar donde copia listas o valores grandes. Los motivos están en [Seguridad](security.md#v1-findings).

## Comisiones

La comisión mínima de una transacción es

```text
comisión = (base_fee + ⌈bytes × fee_per_kb ÷ 1000⌉ + ⌈max_fuel × fee_per_kfuel ÷ 1000⌉) × congestión
```

con los parámetros predeterminados de The Coin `base_fee = 1 000`, `fee_per_kb = 10 000` y `fee_per_kfuel = 1 000` motes. El multiplicador de congestión empieza en 1× y se ajusta con el uso de los bloques; la parte por encima de 1× se quema. Las prioridades de la billetera `low | normal | high | urgent` pagan 1×, 1,25×, ≥ 2× y ≥ 4× el mínimo. Los parámetros los fija la gobernanza, así que consulte los valores actuales de la red antes de depender de cifras exactas.

- Usted paga el combustible que **reserva**. La billetera reserva el combustible medido × 1,3 + 5 000.
- Una transacción **fallida** paga su comisión; todos sus efectos se revierten.
- Las views llamadas por la API son gratuitas y no crean transacciones.

`tccl run` y el playground muestran una estimación con los parámetros predeterminados, por ejemplo:

```text
estimated on-chain cost (default parameters): fee 9247 motes for max_fuel 6497 and ~175 bytes · storage deposit +0 motes
```

## Depósitos de almacenamiento

El estado de un contrato está respaldado por un **depósito reembolsable**:

- El tamaño de un contrato es su código compilado más cada entrada de almacenamiento (clave + valor).
- Depósito exigido = ⌈tamaño ÷ 1 000⌉ × `storage_deposit_per_kb` (predeterminado 100 000 motes, 0,001 TCN por kB iniciado).
- Un despliegue o una llamada que hace **crecer** el estado paga el depósito que falta, hasta el `max_deposit` de la transacción (predeterminado de la billetera: 1 TCN).
- Una llamada que hace **encoger** el estado recibe `depósito × liberado ÷ tamaño anterior`. Guardar un valor predeterminado borra una entrada, así que limpiar datos tiene recompensa.
- `destroy(to)` paga todo el depósito y el saldo a `to`.

El simulador muestra los depósitos como estimaciones y no los descuenta de los saldos ficticios.

## Rendimiento medido

TCCL es un intérprete que recorre el árbol del programa, escrito en Rust. Rust aporta seguridad de memoria y rendimiento predecible; no ejecuta los contratos a velocidad nativa. Lo que importa para la red es que el combustible siga al tiempo de CPU.

Medido con `cargo test -p tccl --release --test perf -- --ignored --nocapture` en un Intel Xeon E5-1620 v2 @ 3,70 GHz (el almacenamiento en memoria del simulador es más rápido que el disco de un nodo, así que las filas de almacenamiento son optimistas):

| Carga | Lenguaje | ns por combustible | Pico de memoria | Bloque de 50 M |
|---|---|---|---|---|
| Bucle aritmético | 1 · 2 | 16,4 · 17,6 | < 0,1 MB | 0,8 s · 0,9 s |
| Escrituras en map | 1 · 2 | 1,4 · 1,2 | 0,4 MB | 0,1 s |
| Cadena de BLAKE3 | 1 · 2 | 5,0 · 4,7 | < 0,1 MB | 0,2 s |
| Construir records | 2 | 13,8 | < 0,1 MB | 0,7 s |
| Copiar una lista de 4 000 textos (adversarial) | 1 · 2 | **2 109** · 12,2 | 0,3 MB | **105 s** · 0,6 s |
| Decodificar una lista guardada (adversarial) | 1 · 2 | **1 656** · 21,1 | 0,4 MB | **83 s** · 1,1 s |
| Retener listas locales grandes (adversarial) | 1 · 2 | 10,1 · 6,4 | **65,6 MB** · 16,8 MB (límite) | 0,5 s · 0,3 s |

Sus cifras serán distintas; ejecute `tccl bench` para una comprobación rápida en su máquina.

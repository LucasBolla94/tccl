# Versiones

TCCL tiene dos números de versión independientes:

- la **versión del lenguaje** (1, 2, …) decide cómo se compila y ejecuta el código, y forma parte del consenso;
- la **versión de la herramienta** (`tccl 0.3.0`) es la versión del compilador, la CLI, el simulador y el playground, con versionado semántico.

## Versiones del lenguaje

| | Versión 1 | Versión 2 |
|---|---|---|
| Estado | En uso en The Coin v0.2.0 | Publicada en tccl 0.3.0; **aún no activa en la red** |
| Compilador | Copia congelada de thecoin@8f620ea (`src/v1`) | `src/parser.rs`, `src/checker.rs` |
| Codificación del programa | `version, name, states, events, functions` | Campos de la versión 1 + `records, enums, interfaces, roles, access, modules, effects` |
| Llamadas entre contratos, actualizaciones | — | ✓ |

### Qué agrega la versión 2

- Records con campos con nombre; enums con transiciones permitidas verificadas en cada escritura.
- Roles, `grant`, `revoke`, `only`; eventos automáticos `RoleGranted`/`RoleRevoked`.
- Interfaces, llamadas entre contratos, `with value`, `origin`; reentrada siempre rechazada; profundidad de contratos 8.
- Módulos (`use`, secciones `module`) y la biblioteca estándar `std.token`, `std.items`, `std.payments`.
- `upgrade()` y el modelo de autoridad de actualización.
- Funciones integradas `mul_div`, `isqrt`, `pow`, `code_hash`, `is_contract`, `is_final`; `to_text` de enums.
- `payable` aceptado antes o después del tipo de retorno.
- Seguridad: límite de memoria de 16 MiB por transacción; copias y valores leídos del almacenamiento cobrados por asignación; `xs[i]` y `len(xs)` ya no copian listas locales.
- Efectos (lo que un contrato puede hacer) y reglas de acceso registrados en el programa para billeteras y exploradores.

### Compatibilidad del código de la versión 1

Todo contrato válido de la versión 1 compila como versión 2, con funciones, estado y eventos idénticos, y se ejecuta con resultados, almacenamiento, eventos y pagos idénticos. Dos cosas pueden cambiar:

- **combustible** — solo donde el contrato copia listas o valores grandes (la versión 2 cobra por asignación y a veces menos porque `xs[i]` ya no copia);
- **nombres** — nada: las palabras nuevas son contextuales, así que un contrato de la versión 1 que usa `to`, `from`, `record` u `only` como nombres sigue compilando.

Un programa compilado como versión 2 se ejecuta con las reglas de la versión 2 (límite de memoria, precios); su disposición de almacenamiento es la misma que la del programa de la versión 1.

## Política de compatibilidad

1. Una versión del lenguaje, una vez en uso, queda **congelada**: la salida del compilador, los mensajes de error, los precios de combustible y el comportamiento en ejecución nunca cambian, para que todo nodo pueda volver a ejecutar la historia.
2. Las mejoras y correcciones que cambian el comportamiento observable salen como **nueva versión del lenguaje**, activada para nuevos despliegues a una altura de activación de la red.
3. Los contratos conservan la versión del lenguaje con la que se desplegaron. Un contrato solo pasa a una versión más nueva mediante una actualización explícita de su autoridad.
4. Las codificaciones solo crecen: las nuevas variantes de enum van al final; las nuevas secciones del programa solo existen en versiones nuevas.
5. Los módulos estándar quedan congelados por versión del lenguaje; sus hashes se registran en cada programa.

## Versiones de la herramienta

| Versión | Fecha | Novedades |
|---|---|---|
| 0.3.0 | 2026-09 | Repositorio propio; versión 2 del lenguaje; CLI `new/test/explain/bundle/bench/upgrade`; playground en WebAssembly; instaladores para Linux y Windows; documentación en inglés, portugués y español; pruebas diferenciales contra el motor de la red |
| 0.2.0 | 2026-09 | Versión 1 del lenguaje dentro del repositorio `thecoin` (The Coin v0.2.0) |

La lista completa de cambios está en [CHANGELOG.md](https://github.com/LucasBolla94/tccl/blob/main/CHANGELOG.md). El plan para activar la versión 2 en The Coin — incluidos ABI y estado versionados, combustible, contratos antiguos, reejecución de la historia, pruebas, activación y recuperación — está en [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

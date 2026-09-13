# Arquitectura

## Repositorio

```text
crates/tccl/          compilador, máquina virtual, simulador, biblioteca estándar (crate de biblioteca)
  src/v1/             compilador congelado de la versión 1 del lenguaje (lexer, parser, checker)
  src/parser.rs       parser de la versión 2
  src/checker.rs      verificador de tipos y compilador de la versión 2
  src/program.rs      formato del programa compilado (Borsh, crítico para el consenso)
  src/vm.rs           intérprete determinista, combustible, interfaz del host, llamadas entre contratos
  src/modules.rs      módulos estándar y unión de módulos locales
  src/upgrade.rs      reglas de compatibilidad de actualizaciones
  src/sim.rs          cadena en memoria usada por la CLI, las pruebas y el playground
  src/scenario.rs     ejecutor de escenarios de prueba
  src/diagnostics.rs  explicaciones de errores en inglés, portugués y español
  src/ring.rs         firmas en anillo vinculables (bLSAG, Ristretto255)
  std/                módulos estándar escritos en TCCL (token, items, payments)
crates/tccl-cli/      el comando `tccl`
crates/tccl-wasm/     el motor compilado a WebAssembly para el playground
crates/tccl-compat/   pruebas diferenciales contra el motor en uso en The Coin
examples/             contratos y escenarios (las recetas probadas)
docs/{en,pt-BR,es}/   esta documentación (contenido)
site/                 generador del sitio (build.mjs), tema (diseño) y código del playground
installer/            install.sh, install.ps1, definición del MSI (WiX)
```

## Del código a la ejecución

```text
código ──► lexer ──► parser ──► checker ──► Program ──► bytes Borsh (guardados en la cadena)
                                               │
                        llamada ──► VM (combustible, límites) ◄──► Host (almacenamiento, saldos, eventos, llamadas)
```

1. **Lexer** — convierte el texto en tokens; la sangría se convierte en `Indent`/`Dedent`; las tabulaciones se rechazan; el código se limita a 48 000 bytes. Ambas versiones comparten el lexer congelado de la versión 1: la versión 2 no agrega tokens, solo palabras contextuales.
2. **Parser** — descenso recursivo con anidamiento (32), cadenas de operadores (64) y profundidad de expresión (128) acotados, para que una entrada hostil no desborde la pila de un nodo.
3. **Checker** — resuelve cada nombre a una posición o índice, verifica tipos, caminos de retorno, pureza de las views, reglas de payable, permisos, transiciones y encapsulamiento de módulos, y genera el **programa**: un árbol totalmente resuelto sin búsqueda de nombres en ejecución.
4. **Program** — codificado en Borsh. La codificación es crítica para el consenso: las variantes de enum solo se agregan al final.
5. **Máquina virtual** — recorre el árbol del programa cobrando combustible en cada paso y solo habla con la cadena mediante el trait `Host`.

## Versiones del lenguaje dentro del motor

`compile(source, options)` elige según `options.version`:

- **Versión 1** usa `src/v1/`, una copia byte a byte del compilador de `thecoin` en el commit `8f620ea`. Nunca debe cambiar de comportamiento: los nodos vuelven a ejecutar despliegues históricos compilando su código otra vez. El crate `tccl-compat` compila más de 10 000 códigos (ejemplos, mutaciones, programas aleatorios) con el crate de referencia y con la copia congelada y exige bytes idénticos y mensajes y posiciones de error idénticos; luego ejecuta miles de llamadas en tres motores — referencia, versión 1 congelada y código de la versión 1 compilado como versión 2 — y exige resultados, almacenamiento, eventos y pagos idénticos.
- **Versión 2** usa `src/parser.rs` y `src/checker.rs`. Es un superconjunto: todo código válido de la versión 1 compila, y las funciones, el estado y los eventos resultantes son idénticos.

El formato del programa sigue la misma regla. Un programa de la versión 1 se codifica exactamente como antes: `version, name, states, events, functions`. Un programa de la versión 2 agrega `records, enums, interfaces, roles, access, modules, effects`. Las nuevas instrucciones, expresiones, funciones integradas y tipos son variantes de enum **agregadas**, así que toda construcción de la versión 1 conserva su codificación, y el decodificador rechaza versiones desconocidas.

En la VM, lo que cambia entre versiones depende de `program.version`: el límite de memoria, el precio de las copias por asignación, los caminos de `xs[i]`/`len(xs)` sin copia y las propias instrucciones de la versión 2. Los programas de la versión 1 siguen los caminos originales.

## La interfaz del host

```rust
pub trait Host {
    fn storage_read(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, VmError>;
    fn storage_write(&mut self, key: &[u8], value: Option<Vec<u8>>) -> Result<(), VmError>;
    fn balance(&mut self) -> Result<u64, VmError>;
    fn send(&mut self, to: &[u8; 20], amount: u64) -> Result<(), VmError>;
    fn emit(&mut self, event: &str, fields: Vec<(String, Value)>) -> Result<(), VmError>;
    fn storage_items(&mut self) -> Result<u64, VmError>;
    fn destroy(&mut self, to: &[u8; 20]) -> Result<(), VmError>;
    // versión 2 (las implementaciones predeterminadas rechazan, así que los hosts actuales siguen compilando)
    fn enter_contract(&mut self, caller: &[u8; 20], callee: &[u8; 20], value: u64)
        -> Result<Option<(Arc<Program>, usize)>, VmError>;
    fn leave_contract(&mut self) -> Result<(), VmError>;
    fn contract_info(&mut self, addr: &[u8; 20]) -> Result<Option<ContractInfo>, VmError>;
}
```

- Las claves de almacenamiento son locales de cada contrato: `[0, var]` escalares, `[1, var, clave]` entradas de map, `[2, var]` longitud de lista, `[3, var, índice]` elementos de lista. El host las separa por contrato y mantiene cada escritura en una capa que se puede revertir.
- **Llamadas entre contratos:** la VM verifica la reentrada (la lista de contratos en ejecución), la profundidad de contratos (8) y de llamadas (16), cobra combustible y pide al host `enter_contract` — que mueve el valor adjunto y convierte al llamado en el contrato actual para almacenamiento, saldo, eventos y envíos —, ejecuta el llamado en una VM hija que comparte los presupuestos de combustible y memoria y llama a `leave_contract`. La VM nunca captura el fallo del llamado: se propaga y el host descarta la capa entera de la transacción. Por eso la atomicidad no necesita snapshots anidados.
- **En The Coin**, `crates/core/src/programs.rs` implementa `Host` sobre la capa de estado. Integrar la versión 2 significa implementar los tres métodos nuevos y la transacción de actualización; vea [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

## Simulador, CLI y playground

- `sim.rs` implementa `Host` con maps en memoria, guarda un snapshot de la cadena antes de cada transacción y lo restaura si falla. Agrega la autoridad de actualización, la decodificación del estado para mostrarlo y las estimaciones de comisión y depósito con los parámetros predeterminados.
- La CLI (`tccl-cli`) y el ejecutor de escenarios son capas finas sobre el simulador.
- `tccl-wasm` compila el mismo crate a `wasm32-unknown-unknown`, sin imports, y expone una interfaz de solicitud/respuesta en JSON. El playground lo carga en un Web Worker y termina el worker si una solicitud supera los cinco segundos.

## Lista de determinismo para quien contribuye

- Nada de coma flotante, iteración de `HashMap`, relojes, aleatoriedad ni hilos en el compilador o la VM.
- Cada bucle de la VM cobra combustible; cada asignación está acotada o se paga.
- Nunca cambie `src/v1/`, el orden de las variantes de enum, los precios de combustible ni los mensajes de error de una versión existente del lenguaje. Cree una versión nueva.
- Agregue una prueba para cada nuevo modo de fallo y ejecute `cargo test --workspace --release` (incluye las pruebas diferenciales).

## Sitio web

`site/build.mjs` (sin dependencias) genera el sitio a partir del **contenido** — `docs/<idioma>/*.md`, `site/content/*.json` y los archivos de ejemplo — y del **diseño** — `site/theme/` (plantilla y CSS) y `site/playground/`. Los bloques de código que incluyen archivos de `examples/` se leen al generar, así que la documentación muestra exactamente el código probado.

# TCCL — The Coin Cloud Language

[English](README.md) · [Português](README.pt-BR.md) · **Sitio y playground: https://tccl.the-coin.cloud**

TCCL es el lenguaje de contratos inteligentes de [The Coin](https://the-coin.cloud). Se lee como Python, pero cada valor tiene un tipo declarado, cada paso cuesta combustible, la aritmética se verifica y el mismo código da exactamente el mismo resultado en todos los nodos. El compilador forma parte del consenso: un despliegue lleva el código fuente y cada nodo lo compila.

## Qué hay aquí

| | |
|---|---|
| `crates/tccl` | Compilador (versiones 1 y 2 del lenguaje), VM determinista, simulador, biblioteca estándar, diagnósticos |
| `crates/tccl-cli` | El comando `tccl`: `new`, `check`, `abi`, `bundle`, `run`, `test`, `explain`, `bench`, `ring` |
| `crates/tccl-wasm` | El mismo motor compilado a WebAssembly para el playground |
| `crates/tccl-compat` | Pruebas diferenciales contra el motor en uso en The Coin v0.2.0 |
| `examples/` | Contratos y escenarios — las recetas probadas |
| `docs/en`, `docs/pt-BR`, `docs/es` | Documentación completa en tres idiomas |
| `site/` | Generador del sitio, tema y playground |
| `installer/` | Instaladores para Linux y Windows |
| [`implant-the-coin-language.md`](implant-the-coin-language.md) | Plan para activar la versión 2 del lenguaje en The Coin |

## Versiones del lenguaje

- **Versión 1** se ejecuta en The Coin v0.2.0. Este repositorio mantiene su compilador congelado byte a byte y lo compara con el motor desplegado.
- **Versión 2** agrega records, enums con transiciones permitidas, roles y `only`, interfaces y llamadas entre contratos (sin reentrada, atómicas), módulos y biblioteca estándar (`std.token`, `std.items`, `std.payments`), autoridad de actualización, `mul_div`/`isqrt`/`pow` y refuerzo de memoria y combustible. **Aún no está activa en la red.**

## Instalar

```sh
curl -fsSL https://tccl.the-coin.cloud/install.sh | sh                              # Linux
powershell -ExecutionPolicy Bypass -c "irm https://tccl.the-coin.cloud/install.ps1 | iex"  # Windows
cargo install --git https://github.com/LucasBolla94/tccl tccl-cli                  # desde el código
```

Luego: `tccl new hola && cd hola && tccl test`. Lea [Su primer contrato](docs/es/tutorial.md).

## Licencia

Licencia doble MIT o Apache 2.0, a su elección. Extraído de [LucasBolla94/thecoin](https://github.com/LucasBolla94/thecoin) con su historial preservado; vea [NOTICE](NOTICE).

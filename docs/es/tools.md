# Herramientas

## Instalar {#install}

| Sistema | Comando |
|---|---|
| Linux (x86_64, aarch64) | `curl -fsSL https://tccl.the-coin.cloud/install.sh \| sh` |
| Windows 10/11 (x86_64) | `powershell -ExecutionPolicy Bypass -c "irm https://tccl.the-coin.cloud/install.ps1 \| iex"` |
| Instalador de Windows | `tccl-x86_64-pc-windows-msvc.msi` en las [versiones](https://github.com/LucasBolla94/tccl/releases) |
| Paquete Debian/Ubuntu | `tccl_*.deb` de las versiones, luego `sudo apt install ./tccl_*.deb` |
| Desde el código (cualquier sistema con Rust 1.88+) | `cargo install --git https://github.com/LucasBolla94/tccl tccl-cli` |

Los scripts descargan de las versiones de GitHub y **verifican el checksum SHA-256** listado en `SHA256SUMS` antes de instalar. El script de Linux instala en `/usr/local/bin` si tiene permiso de escritura y, si no, en `~/.local/bin`; `TCCL_INSTALL_DIR` y `TCCL_VERSION` lo cambian. El script de Windows instala para el usuario actual en `%LOCALAPPDATA%\Programs\tccl` y lo agrega al `PATH` del usuario.

> Los binarios de Windows aún no tienen firma de código, así que Windows SmartScreen puede advertir de un editor desconocido. Compare el SHA-256 del archivo con `SHA256SUMS` de la página de la versión.

## Referencia de la línea de comandos

| Comando | Descripción |
|---|---|
| `tccl new <nombre>` | Crea una carpeta con `contract.tccl` y `contract.scenario` |
| `tccl check <archivo>` | Compila y muestra la interfaz, roles, reglas `only` y qué puede hacer el contrato |
| `tccl abi <archivo>` | La interfaz en JSON (funciones, eventos, estado, records, enums, roles, módulos, efectos, hash del código) |
| `tccl bundle <archivo> [-o salida]` | Agrega módulos locales (`use mylib` → `mylib.tccl`) para producir un solo archivo desplegable |
| `tccl run <archivo> [opciones] deploy [args…]` | Despliega en el simulador |
| `tccl run <archivo> [opciones] call <función> [args…]` | Llama una action |
| `tccl run <archivo> [opciones] view <función> [args…]` | Consulta una view |
| `tccl run <archivo> [opciones] upgrade [args…]` | Reemplaza el código del último contrato desplegado (solo la autoridad de actualización) |
| `tccl run <archivo> [opciones] authority <@cuenta\|none>` | Cede o renuncia a la autoridad de actualización |
| `tccl run <archivo> [opciones] state` | Estado decodificado, saldo, versión del código y autoridad |
| `tccl test [archivos o carpetas]` | Ejecuta archivos `*.scenario` (predeterminado: carpeta actual), `-v` para ver el registro |
| `tccl explain <código>` | Explica un código de error como `C006` o `R018` |
| `tccl docs errors` | Imprime la referencia de errores en Markdown |
| `tccl bench` | Mide el motor en esta máquina |
| `tccl ring keygen [--seed <hex32>]` | Crea un par de claves de anillo para pools de privacidad |
| `tccl ring sign --secret <hex> --ring <pk,…> --index <i> --message <0xhex>` | Produce una firma en anillo |
| `tccl version` | Versiones de la herramienta y del lenguaje |

Opciones comunes: `--language 1|2` (predeterminado 2) y `--lang en|pt|es` para las explicaciones (o la variable de entorno `TCCL_LANG`).

Opciones de `run`: `--state <archivo>` (predeterminado `tccl-state.json`), `--from <nombre>` (predeterminado `alice`), `--value 5tcn`, `--height <n>`, `--contract <hex>`, `--final`.

Los **argumentos** siguen los tipos declarados de los parámetros: `42` o `2.5tcn` (int), `true`, `"texto"`, `0xabcd`, `tc1…`, `@nombre` para una cuenta ficticia, `$prefijo-hex` para un contrato simulado, `[a, b]` (listas), `{campo: valor}` (records) y el nombre de una variante (enums).

## Escenarios {#scenarios}

Un escenario es una prueba en texto plano. Cada línea es un comando; `#` empieza un comentario.

| Comando | Significado |
|---|---|
| `deploy <archivo> as <nombre> [args…] [--from A] [--value X] [--final] [--language 1\|2]` | Despliega y nombra el contrato |
| `call <nombre> <función> [args…] [--from A] [--value X]` | Llama una action |
| `view <nombre> <función> [args…]` | Consulta una view |
| `upgrade <nombre> <archivo> [args…] [--from A]` | Actualiza un contrato |
| `authority <nombre> <@cuenta\|none> [--from A]` | Cambia la autoridad de actualización |
| `height <n>` · `advance <n>` | Fija o avanza la altura del bloque |
| `expect ok` | El último comando tuvo éxito |
| `expect fail ["texto"]` | Falló (y el mensaje contiene el texto) |
| `expect result <valor>` | Devolvió ese valor (misma sintaxis que los argumentos) |
| `expect event <Nombre>` | Emitió ese evento |
| `expect balance <@cuenta\|nombre> <monto>` | Una cuenta o contrato tiene ese saldo |
| `expect state <nombre> <variable> <valor>` | Una variable de estado muestra ese valor |

En los argumentos, `$nombre` es la dirección de un contrato desplegado antes en el escenario y `@nombre` una cuenta ficticia (también funciona dentro de listas y records). Cada escenario empieza en una cadena simulada nueva donde cada cuenta tiene 1 000 000 TCN.

## Playground

El [playground](/es/playground/) ejecuta el mismo compilador, máquina virtual y simulador, compilados a WebAssembly, dentro de un Web Worker en su navegador.

- **Editor:** resaltado de sintaxis, línea y columna, Tab inserta cuatro espacios, errores resaltados con explicación y corrección con un clic para "¿quiso decir?".
- **Ejecutar:** despliegue con cuentas ficticias, complete argumentos, llame actions y consulte views, actualice y cambie la autoridad.
- **Estado:** saldos, depósito, versión del código, autoridad y cada variable de estado, con los cambios resaltados.
- **Actividad:** cada transacción con su resultado o error explicado, eventos, combustible, comisión y depósito estimados y la lista de cambios de estado.
- **Escenario:** ejecuta un escenario sobre el código del editor.
- **Sesión:** fija la altura del bloque, exporta e importa la cadena simulada, reinicia.
- **Archivos:** abra y guarde archivos `.tccl`, o suelte uno sobre el editor.

Nunca pide claves ni frases de recuperación y nunca contacta una red. Las solicitudes que tardan más de cinco segundos se detienen; cada llamada tiene como máximo 5 000 000 de combustible y 16 MiB.

## Simulación, testnet y mainnet

| | Simulación (`tccl`, playground) | Testnet | Mainnet |
|---|---|---|---|
| Dinero | Ficticio, 1 000 000 TCN por cuenta | Monedas de prueba sin valor | TCN real |
| Direcciones | `tcr1…` | `tct1…` | `tc1…` |
| Claves | Ninguna | Su billetera | Su billetera |
| Versión del lenguaje | 1 o 2 | 1 (v0.2.0) | 1 (v0.2.0) |
| Herramienta | `tccl` | `thecoin-wallet --network testnet` | `thecoin-wallet` |

## Testnet {#testnet}

Despliegue con la billetera del software del nodo (vea [the-coin.cloud](https://the-coin.cloud/docs.html)). Verifique antes el contrato con `--language 1`, porque la red ejecuta la versión 1 del lenguaje:

```sh
tccl check --language 1 contract.tccl
thecoin-wallet --network testnet contract deploy contract.tccl [args de init…] [--value TCN] [--max-fuel N] [--max-deposit TCN]
thecoin-wallet --network testnet contract invoke <dirección> <función> [args…] [--value TCN]
thecoin-wallet --network testnet contract view <dirección> <función> [args…]
thecoin-wallet --network testnet contract program <dirección>
```

La billetera simula cada llamada sobre el estado actual antes y no envía una llamada que fallaría.

## Mainnet {#mainnet}

Los mismos comandos sin `--network testnet` usan TCN real. Antes de desplegar: siga la [lista de control de seguridad](security.md#checklist), ejecute el contrato en testnet, publique su código y consiga una revisión. En The Coin v0.2.0 el código desplegado no se puede cambiar.

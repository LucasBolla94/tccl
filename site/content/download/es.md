# Descargar tccl {{version}}

`tccl` compila, verifica, simula y prueba contratos TCCL en su computadora. Nunca maneja claves: para desplegar use `thecoin-wallet`.

## Linux

```sh
curl -fsSL https://tccl.the-coin.cloud/install.sh | sh
```

Funciona en x86_64 y aarch64 (binarios estáticos musl). El script verifica el checksum SHA-256 antes de instalar en `/usr/local/bin` o `~/.local/bin`.

## Windows

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://tccl.the-coin.cloud/install.ps1 | iex"
```

O descargue `tccl-x86_64-pc-windows-msvc.msi` (instalador por usuario) o el `.zip` desde la página de la versión. Los binarios aún no tienen firma de código: Windows SmartScreen puede mostrar "editor desconocido". Verifique el archivo con `SHA256SUMS`.

## Todos los archivos

Cada versión en [GitHub](https://github.com/LucasBolla94/tccl/releases/latest) contiene:

| Archivo | Para |
|---|---|
| `tccl-x86_64-unknown-linux-musl.tar.gz` | Linux x86_64 |
| `tccl-aarch64-unknown-linux-musl.tar.gz` | Linux ARM64 (Raspberry Pi 4/5, servidores ARM) |
| `tccl_*_amd64.deb` | Debian y Ubuntu |
| `tccl-x86_64-pc-windows-msvc.zip` · `.msi` | Windows 10/11 x86_64 |
| `tccl-engine.wasm` | El motor WebAssembly que usa el playground |
| `SHA256SUMS` | Checksums de todos los archivos |

## Desde el código fuente

```sh
cargo install --git https://github.com/LucasBolla94/tccl tccl-cli
```

Requiere Rust 1.88 o posterior. Luego lea [Su primer contrato](/es/docs/tutorial.html).

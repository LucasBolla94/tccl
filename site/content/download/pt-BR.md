# Baixar o tccl {{version}}

O `tccl` compila, verifica, simula e testa contratos TCCL no seu computador. Ele nunca lida com chaves: para publicar, use o `thecoin-wallet`.

## Linux

```sh
curl -fsSL https://tccl.the-coin.cloud/install.sh | sh
```

Funciona em x86_64 e aarch64 (binários estáticos musl). O script confere o checksum SHA-256 antes de instalar em `/usr/local/bin` ou `~/.local/bin`.

## Windows

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://tccl.the-coin.cloud/install.ps1 | iex"
```

Ou baixe `tccl-x86_64-pc-windows-msvc.msi` (instalador por usuário) ou o `.zip` na página da versão. Os binários ainda não têm assinatura de código: o Windows SmartScreen pode mostrar "editor desconhecido". Confira o arquivo com o `SHA256SUMS`.

## Todos os arquivos

Cada versão no [GitHub](https://github.com/LucasBolla94/tccl/releases/latest) traz:

| Arquivo | Para |
|---|---|
| `tccl-x86_64-unknown-linux-musl.tar.gz` | Linux x86_64 |
| `tccl-aarch64-unknown-linux-musl.tar.gz` | Linux ARM64 (Raspberry Pi 4/5, servidores ARM) |
| `tccl_*_amd64.deb` | Debian e Ubuntu |
| `tccl-x86_64-pc-windows-msvc.zip` · `.msi` | Windows 10/11 x86_64 |
| `tccl-engine.wasm` | O motor WebAssembly usado pelo playground |
| `SHA256SUMS` | Checksums de todos os arquivos |

## A partir do código-fonte

```sh
cargo install --git https://github.com/LucasBolla94/tccl tccl-cli
```

Requer Rust 1.88 ou mais novo. Depois leia [Seu primeiro contrato](/pt-BR/docs/tutorial.html).

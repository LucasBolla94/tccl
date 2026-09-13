# Download tccl {{version}}

`tccl` compiles, checks, simulates and tests TCCL contracts on your computer. It never handles keys: deploy with `thecoin-wallet`.

## Linux

```sh
curl -fsSL https://tccl.the-coin.cloud/install.sh | sh
```

Supports x86_64 and aarch64 (static musl binaries). The script verifies the SHA-256 checksum before installing into `/usr/local/bin` or `~/.local/bin`.

## Windows

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://tccl.the-coin.cloud/install.ps1 | iex"
```

Or download `tccl-x86_64-pc-windows-msvc.msi` (per-user installer) or the `.zip` from the release page. The binaries are not code-signed yet: Windows SmartScreen may show "unknown publisher". Verify the file against `SHA256SUMS`.

## All files

Every release on [GitHub](https://github.com/LucasBolla94/tccl/releases/latest) contains:

| File | For |
|---|---|
| `tccl-x86_64-unknown-linux-musl.tar.gz` | Linux x86_64 |
| `tccl-aarch64-unknown-linux-musl.tar.gz` | Linux ARM64 (Raspberry Pi 4/5, ARM servers) |
| `tccl_*_amd64.deb` | Debian and Ubuntu |
| `tccl-x86_64-pc-windows-msvc.zip` · `.msi` | Windows 10/11 x86_64 |
| `tccl-engine.wasm` | The WebAssembly engine used by the playground |
| `SHA256SUMS` | Checksums of every file |

## From source

```sh
cargo install --git https://github.com/LucasBolla94/tccl tccl-cli
```

Requires Rust 1.88 or newer. Then read [Your first contract](/docs/tutorial.html).

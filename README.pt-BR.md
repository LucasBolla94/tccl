# TCCL — The Coin Cloud Language

[English](README.md) · [Español](README.es.md) · **Site e playground: https://tccl.the-coin.cloud**

A TCCL é a linguagem de contratos inteligentes da [The Coin](https://the-coin.cloud). Ela se lê como Python, mas todo valor tem tipo declarado, todo passo custa combustível, a aritmética é verificada e o mesmo código dá exatamente o mesmo resultado em todos os nós. O compilador faz parte do consenso: uma publicação leva o código-fonte e cada nó o compila.

## O que há aqui

| | |
|---|---|
| `crates/tccl` | Compilador (versões 1 e 2 da linguagem), VM determinística, simulador, biblioteca padrão, diagnósticos |
| `crates/tccl-cli` | O comando `tccl`: `new`, `check`, `abi`, `bundle`, `run`, `test`, `explain`, `bench`, `ring` |
| `crates/tccl-wasm` | O mesmo motor compilado para WebAssembly, usado pelo playground |
| `crates/tccl-compat` | Testes diferenciais contra o motor em uso na The Coin v0.2.0 |
| `examples/` | Contratos e cenários — as receitas testadas |
| `docs/en`, `docs/pt-BR`, `docs/es` | Documentação completa em três idiomas |
| `site/` | Gerador do site, tema e playground |
| `installer/` | Instaladores para Linux e Windows |
| [`implant-the-coin-language.md`](implant-the-coin-language.md) | Plano para ativar a versão 2 da linguagem na The Coin |

## Versões da linguagem

- **Versão 1** roda na The Coin v0.2.0. Este repositório mantém o compilador dela congelado byte a byte e o confere contra o motor publicado.
- **Versão 2** acrescenta records, enums com transições permitidas, papéis e `only`, interfaces e chamadas entre contratos (sem reentrada, atômicas), módulos e biblioteca padrão (`std.token`, `std.items`, `std.payments`), autoridade de upgrade, `mul_div`/`isqrt`/`pow` e reforço de memória e combustível. **Ainda não está ativa na rede.**

## Instalar

```sh
curl -fsSL https://tccl.the-coin.cloud/install.sh | sh                              # Linux
powershell -ExecutionPolicy Bypass -c "irm https://tccl.the-coin.cloud/install.ps1 | iex"  # Windows
cargo install --git https://github.com/LucasBolla94/tccl tccl-cli                  # a partir do código
```

Depois: `tccl new ola && cd ola && tccl test`. Leia [Seu primeiro contrato](docs/pt-BR/tutorial.md).

## Licença

Licença dupla MIT ou Apache 2.0, à sua escolha. Extraído de [LucasBolla94/thecoin](https://github.com/LucasBolla94/thecoin) com o histórico preservado; veja [NOTICE](NOTICE).

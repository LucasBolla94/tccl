# Ferramentas

## Instalar {#install}

| Sistema | Comando |
|---|---|
| Linux (x86_64, aarch64) | `curl -fsSL https://tccl.the-coin.cloud/install.sh \| sh` |
| Windows 10/11 (x86_64) | `powershell -ExecutionPolicy Bypass -c "irm https://tccl.the-coin.cloud/install.ps1 \| iex"` |
| Instalador Windows | `tccl-x86_64-pc-windows-msvc.msi` nas [versões](https://github.com/LucasBolla94/tccl/releases) |
| Pacote Debian/Ubuntu | `tccl_*.deb` nas versões, depois `sudo apt install ./tccl_*.deb` |
| A partir do código (qualquer sistema com Rust 1.88+) | `cargo install --git https://github.com/LucasBolla94/tccl tccl-cli` |

Os scripts baixam das versões do GitHub e **conferem o checksum SHA-256** listado em `SHA256SUMS` antes de instalar. O script do Linux instala em `/usr/local/bin` quando há permissão de escrita, senão em `~/.local/bin`; `TCCL_INSTALL_DIR` e `TCCL_VERSION` mudam isso. O script do Windows instala para o usuário atual em `%LOCALAPPDATA%\Programs\tccl` e adiciona ao `PATH` do usuário.

> Os binários do Windows ainda não têm assinatura de código, então o Windows SmartScreen pode avisar sobre editor desconhecido. Compare o SHA-256 do arquivo com o `SHA256SUMS` da página da versão.

## Referência da linha de comando

| Comando | Descrição |
|---|---|
| `tccl new <nome>` | Cria uma pasta com `contract.tccl` e `contract.scenario` |
| `tccl check <arquivo>` | Compila e mostra a interface, papéis, regras `only` e o que o contrato pode fazer |
| `tccl abi <arquivo>` | A interface em JSON (funções, eventos, estado, records, enums, papéis, módulos, efeitos, hash do código) |
| `tccl bundle <arquivo> [-o saída]` | Junta módulos locais (`use mylib` → `mylib.tccl`) num único arquivo publicável |
| `tccl run <arquivo> [opções] deploy [args…]` | Publica no simulador |
| `tccl run <arquivo> [opções] call <função> [args…]` | Chama uma action |
| `tccl run <arquivo> [opções] view <função> [args…]` | Consulta uma view |
| `tccl run <arquivo> [opções] upgrade [args…]` | Troca o código do último contrato publicado (só a autoridade de upgrade) |
| `tccl run <arquivo> [opções] authority <@conta\|none>` | Passa adiante ou renuncia à autoridade de upgrade |
| `tccl run <arquivo> [opções] state` | Estado decodificado, saldo, versão do código e autoridade |
| `tccl test [arquivos ou pastas]` | Roda arquivos `*.scenario` (padrão: pasta atual), `-v` para ver o registro |
| `tccl explain <código>` | Explica um código de erro como `C006` ou `R018` |
| `tccl docs errors` | Imprime a referência de erros em Markdown |
| `tccl bench` | Mede o motor nesta máquina |
| `tccl ring keygen [--seed <hex32>]` | Cria um par de chaves de anel para pools de privacidade |
| `tccl ring sign --secret <hex> --ring <pk,…> --index <i> --message <0xhex>` | Gera uma assinatura em anel |
| `tccl version` | Versões da ferramenta e da linguagem |

Opções comuns: `--language 1|2` (padrão 2) e `--lang en|pt|es` para as explicações (ou a variável de ambiente `TCCL_LANG`).

Opções do `run`: `--state <arquivo>` (padrão `tccl-state.json`), `--from <nome>` (padrão `alice`), `--value 5tcn`, `--height <n>`, `--contract <hex>`, `--final`.

**Argumentos** seguem os tipos declarados dos parâmetros: `42` ou `2.5tcn` (int), `true`, `"texto"`, `0xabcd`, `tc1…`, `@nome` para uma conta fictícia, `$prefixo-hex` para um contrato simulado, `[a, b]` (listas), `{campo: valor}` (records) e o nome de uma variante (enums).

## Cenários {#scenarios}

Um cenário é um teste em texto simples. Cada linha é um comando; `#` começa um comentário.

| Comando | Significado |
|---|---|
| `deploy <arquivo> as <nome> [args…] [--from A] [--value X] [--final] [--language 1\|2]` | Publica e dá nome ao contrato |
| `call <nome> <função> [args…] [--from A] [--value X]` | Chama uma action |
| `view <nome> <função> [args…]` | Consulta uma view |
| `upgrade <nome> <arquivo> [args…] [--from A]` | Atualiza um contrato |
| `authority <nome> <@conta\|none> [--from A]` | Troca a autoridade de upgrade |
| `height <n>` · `advance <n>` | Define ou avança a altura do bloco |
| `expect ok` | O último comando deu certo |
| `expect fail ["texto"]` | Falhou (e a mensagem contém o texto) |
| `expect result <valor>` | Devolveu esse valor (mesma sintaxe dos argumentos) |
| `expect event <Nome>` | Emitiu esse evento |
| `expect balance <@conta\|nome> <valor>` | Uma conta ou contrato tem esse saldo |
| `expect state <nome> <variável> <valor>` | Uma variável de estado mostra esse valor |

Nos argumentos, `$nome` é o endereço de um contrato publicado antes no cenário, e `@nome` uma conta fictícia (funciona também dentro de listas e records). Todo cenário começa numa blockchain simulada nova, em que cada conta tem 1 000 000 TCN.

## Playground

O [playground](/pt-BR/playground/) roda o mesmo compilador, máquina virtual e simulador, compilados para WebAssembly, dentro de um Web Worker no seu navegador.

- **Editor:** destaque de sintaxe, linha e coluna, Tab insere quatro espaços, erros destacados com explicação e correção com um clique para "você quis dizer".
- **Executar:** publique com contas fictícias, preencha argumentos, chame actions e consulte views, atualize e troque a autoridade.
- **Estado:** saldos, depósito, versão do código, autoridade e cada variável de estado, com as mudanças destacadas.
- **Atividade:** cada transação com resultado ou erro explicado, eventos, combustível, taxa e depósito estimados e a lista de mudanças de estado.
- **Cenário:** roda um cenário com o código do editor.
- **Sessão:** define a altura do bloco, exporta e importa a blockchain simulada, reinicia.
- **Arquivos:** abra e salve arquivos `.tccl`, ou arraste um para o editor.

Ele nunca pede chaves nem frases de recuperação e nunca acessa uma rede. Pedidos que levam mais de cinco segundos são interrompidos; cada chamada tem no máximo 5 000 000 de combustível e 16 MiB.

## Simulação, testnet e mainnet

| | Simulação (`tccl`, playground) | Testnet | Mainnet |
|---|---|---|---|
| Dinheiro | Fictício, 1 000 000 TCN por conta | Moedas de teste sem valor | TCN real |
| Endereços | `tcr1…` | `tct1…` | `tc1…` |
| Chaves | Nenhuma | Sua carteira | Sua carteira |
| Versão da linguagem | 1 ou 2 | 1 (v0.2.0) | 1 (v0.2.0) |
| Ferramenta | `tccl` | `thecoin-wallet --network testnet` | `thecoin-wallet` |

## Testnet {#testnet}

Publique com a carteira do software do nó (veja [the-coin.cloud](https://the-coin.cloud/docs.html)). Verifique o contrato com `--language 1` antes, porque a rede roda a versão 1 da linguagem:

```sh
tccl check --language 1 contract.tccl
thecoin-wallet --network testnet contract deploy contract.tccl [args do init…] [--value TCN] [--max-fuel N] [--max-deposit TCN]
thecoin-wallet --network testnet contract invoke <endereço> <função> [args…] [--value TCN]
thecoin-wallet --network testnet contract view <endereço> <função> [args…]
thecoin-wallet --network testnet contract program <endereço>
```

A carteira simula cada chamada sobre o estado atual antes e não envia uma chamada que falharia.

## Mainnet {#mainnet}

Os mesmos comandos sem `--network testnet` usam TCN real. Antes de publicar: siga o [checklist de segurança](security.md#checklist), rode o contrato na testnet, publique o código e peça uma revisão. Na The Coin v0.2.0 o código publicado não pode ser alterado.

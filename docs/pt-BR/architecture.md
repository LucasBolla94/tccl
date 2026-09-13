# Arquitetura

## Repositório

```text
crates/tccl/          compilador, máquina virtual, simulador, biblioteca padrão (crate de biblioteca)
  src/v1/             compilador congelado da versão 1 da linguagem (lexer, parser, checker)
  src/parser.rs       parser da versão 2
  src/checker.rs      verificador de tipos e compilador da versão 2
  src/program.rs      formato do programa compilado (Borsh, crítico para o consenso)
  src/vm.rs           interpretador determinístico, combustível, interface do host, chamadas entre contratos
  src/modules.rs      módulos padrão e junção de módulos locais
  src/upgrade.rs      regras de compatibilidade de upgrades
  src/sim.rs          blockchain em memória usada pela CLI, pelos testes e pelo playground
  src/scenario.rs     executor de cenários de teste
  src/diagnostics.rs  explicações de erros em inglês, português e espanhol
  src/ring.rs         assinaturas em anel vinculáveis (bLSAG, Ristretto255)
  std/                módulos padrão escritos em TCCL (token, items, payments)
crates/tccl-cli/      o comando `tccl`
crates/tccl-wasm/     o motor compilado para WebAssembly para o playground
crates/tccl-compat/   testes diferenciais contra o motor em uso na The Coin
examples/             contratos e cenários (as receitas testadas)
docs/{en,pt-BR,es}/   esta documentação (conteúdo)
site/                 gerador do site (build.mjs), tema (design) e código do playground
installer/            install.sh, install.ps1, definição do MSI (WiX)
```

## Do código-fonte à execução

```text
código ──► lexer ──► parser ──► checker ──► Program ──► bytes Borsh (gravados na blockchain)
                                               │
                       chamada ──► VM (combustível, limites) ◄──► Host (armazenamento, saldos, eventos, chamadas)
```

1. **Lexer** — transforma texto em tokens; a indentação vira `Indent`/`Dedent`; tabulações são rejeitadas; o código é limitado a 48 000 bytes. As duas versões compartilham o lexer congelado da versão 1: a versão 2 não acrescenta tokens, só palavras contextuais.
2. **Parser** — descida recursiva com aninhamento (32), cadeias de operadores (64) e profundidade de expressão (128) limitados, para que entrada hostil não estoure a pilha de um nó.
3. **Checker** — resolve cada nome para uma posição ou índice, confere tipos, caminhos de retorno, pureza das views, regras de payable, permissões, transições e encapsulamento de módulos, e gera o **programa**: uma árvore totalmente resolvida, sem busca de nomes durante a execução.
4. **Program** — codificado em Borsh. A codificação é crítica para o consenso: variantes de enum só são acrescentadas no final.
5. **Máquina virtual** — percorre a árvore do programa cobrando combustível a cada passo e só conversa com a blockchain pela trait `Host`.

## Versões da linguagem dentro do motor

`compile(source, options)` escolhe o caminho por `options.version`:

- **Versão 1** usa `src/v1/`, cópia byte a byte do compilador do `thecoin` no commit `8f620ea`. Ela nunca pode mudar de comportamento: os nós reexecutam publicações históricas compilando o código de novo. O crate `tccl-compat` compila mais de 10 000 códigos (exemplos, mutações, programas aleatórios) com o crate de referência e com a cópia congelada e exige bytes idênticos e mensagens e posições de erro idênticas; depois roda milhares de chamadas em três motores — referência, versão 1 congelada e código da versão 1 compilado como versão 2 — e exige resultados, armazenamento, eventos e pagamentos idênticos.
- **Versão 2** usa `src/parser.rs` e `src/checker.rs`. É um superconjunto: todo código válido da versão 1 compila, e as funções, o estado e os eventos resultantes são idênticos.

O formato do programa segue a mesma regra. Um programa da versão 1 é codificado exatamente como antes: `version, name, states, events, functions`. Um programa da versão 2 acrescenta `records, enums, interfaces, roles, access, modules, effects`. Novas instruções, expressões, funções embutidas e tipos são variantes de enum **acrescentadas**, então toda construção da versão 1 mantém sua codificação, e o decodificador rejeita versões desconhecidas.

Na VM, o que muda de uma versão para outra depende de `program.version`: o limite de memória, o preço das cópias por alocação, os caminhos de `xs[i]`/`len(xs)` sem cópia e as próprias instruções da versão 2. Programas da versão 1 seguem os caminhos originais.

## A interface do host

```rust
pub trait Host {
    fn storage_read(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, VmError>;
    fn storage_write(&mut self, key: &[u8], value: Option<Vec<u8>>) -> Result<(), VmError>;
    fn balance(&mut self) -> Result<u64, VmError>;
    fn send(&mut self, to: &[u8; 20], amount: u64) -> Result<(), VmError>;
    fn emit(&mut self, event: &str, fields: Vec<(String, Value)>) -> Result<(), VmError>;
    fn storage_items(&mut self) -> Result<u64, VmError>;
    fn destroy(&mut self, to: &[u8; 20]) -> Result<(), VmError>;
    // versão 2 (as implementações padrão recusam, então os hosts atuais continuam compilando)
    fn enter_contract(&mut self, caller: &[u8; 20], callee: &[u8; 20], value: u64)
        -> Result<Option<(Arc<Program>, usize)>, VmError>;
    fn leave_contract(&mut self) -> Result<(), VmError>;
    fn contract_info(&mut self, addr: &[u8; 20]) -> Result<Option<ContractInfo>, VmError>;
}
```

- As chaves de armazenamento são locais de cada contrato: `[0, var]` escalares, `[1, var, chave]` entradas de map, `[2, var]` tamanho de lista, `[3, var, índice]` itens de lista. O host separa as chaves por contrato e mantém cada escrita numa camada que pode ser revertida.
- **Chamadas entre contratos:** a VM confere reentrada (a lista de contratos em execução), a profundidade de contratos (8) e de chamadas (16), cobra combustível e pede ao host `enter_contract` — que move o valor anexado e torna o chamado o contrato atual para armazenamento, saldo, eventos e envios —, roda o chamado numa VM filha que compartilha os orçamentos de combustível e memória e chama `leave_contract`. A VM nunca captura a falha do chamado: ela se propaga, e o host descarta a camada inteira da transação. Por isso a atomicidade não precisa de snapshots aninhados.
- **Na The Coin**, `crates/core/src/programs.rs` implementa `Host` sobre a camada de estado. Integrar a versão 2 significa implementar os três métodos novos e a transação de upgrade; veja [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

## Simulador, CLI e playground

- `sim.rs` implementa `Host` com maps em memória, tira um snapshot da blockchain antes de cada transação e o restaura em caso de falha. Acrescenta a autoridade de upgrade, a decodificação do estado para exibição e as estimativas de taxa e depósito com os parâmetros padrão.
- A CLI (`tccl-cli`) e o executor de cenários são camadas finas sobre o simulador.
- `tccl-wasm` compila o mesmo crate para `wasm32-unknown-unknown`, sem imports, e expõe uma interface de pedido/resposta em JSON. O playground o carrega num Web Worker e encerra o worker se um pedido passar de cinco segundos.

## Checklist de determinismo para quem contribui

- Nada de ponto flutuante, iteração de `HashMap`, relógios, aleatoriedade ou threads no compilador ou na VM.
- Todo laço da VM cobra combustível; toda alocação é limitada ou paga.
- Nunca altere `src/v1/`, a ordem das variantes de enum, preços de combustível ou mensagens de erro de uma versão existente da linguagem. Crie uma versão nova.
- Acrescente um teste para cada novo modo de falha e rode `cargo test --workspace --release` (inclui os testes diferenciais).

## Site

`site/build.mjs` (sem dependências) gera o site a partir do **conteúdo** — `docs/<idioma>/*.md`, `site/content/*.json` e os arquivos de exemplo — e do **design** — `site/theme/` (layout e CSS) e `site/playground/`. Blocos de código que incluem arquivos de `examples/` são lidos na geração, então a documentação mostra exatamente o código testado.

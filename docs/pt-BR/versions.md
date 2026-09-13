# Versões

A TCCL tem dois números de versão independentes:

- a **versão da linguagem** (1, 2, …) decide como o código compila e executa, e faz parte do consenso;
- a **versão da ferramenta** (`tccl 0.3.0`) é a versão do compilador, da CLI, do simulador e do playground, seguindo o versionamento semântico.

## Versões da linguagem

| | Versão 1 | Versão 2 |
|---|---|---|
| Situação | Em uso na The Coin v0.2.0 | Lançada no tccl 0.3.0; **ainda não ativa na rede** |
| Compilador | Cópia congelada de thecoin@8f620ea (`src/v1`) | `src/parser.rs`, `src/checker.rs` |
| Codificação do programa | `version, name, states, events, functions` | Campos da versão 1 + `records, enums, interfaces, roles, access, modules, effects` |
| Chamadas entre contratos, upgrades | — | ✓ |

### O que a versão 2 acrescenta

- Records com campos nomeados; enums com transições permitidas conferidas em toda gravação.
- Papéis, `grant`, `revoke`, `only`; eventos automáticos `RoleGranted`/`RoleRevoked`.
- Interfaces, chamadas entre contratos, `with value`, `origin`; reentrada sempre rejeitada; profundidade de contratos 8.
- Módulos (`use`, seções `module`) e a biblioteca padrão `std.token`, `std.items`, `std.payments`.
- `upgrade()` e o modelo de autoridade de upgrade.
- Funções embutidas `mul_div`, `isqrt`, `pow`, `code_hash`, `is_contract`, `is_final`; `to_text` de enums.
- `payable` aceito antes ou depois do tipo de retorno.
- Segurança: limite de memória de 16 MiB por transação; cópias e valores lidos do armazenamento cobrados por alocação; `xs[i]` e `len(xs)` não copiam mais listas locais.
- Efeitos (o que um contrato pode fazer) e regras de acesso registrados no programa para carteiras e exploradores.

### Compatibilidade de código da versão 1

Todo contrato válido da versão 1 compila como versão 2, com funções, estado e eventos idênticos, e executa com resultados, armazenamento, eventos e pagamentos idênticos. Duas coisas podem mudar:

- **combustível** — só onde o contrato copia listas ou valores grandes (a versão 2 cobra por alocação e às vezes cobra menos porque `xs[i]` não copia mais);
- **nomes** — nada: as palavras novas são contextuais, então um contrato da versão 1 que usa `to`, `from`, `record` ou `only` como nomes continua compilando.

Um programa compilado como versão 2 executa com as regras da versão 2 (limite de memória, preços); seu layout de armazenamento é o mesmo do programa da versão 1.

## Política de compatibilidade

1. Uma versão da linguagem, depois de em uso, fica **congelada**: a saída do compilador, as mensagens de erro, os preços de combustível e o comportamento na execução nunca mudam, para que todo nó possa reexecutar o histórico.
2. Melhorias e correções que mudam comportamento observável saem como **nova versão da linguagem**, ativada para novas publicações numa altura de ativação da rede.
3. Contratos mantêm a versão da linguagem com que foram publicados. Um contrato só passa para uma versão mais nova por um upgrade explícito da sua autoridade.
4. As codificações só crescem: novas variantes de enum vão para o final; novas seções de programa só existem em versões novas.
5. Módulos padrão ficam congelados por versão da linguagem; seus hashes ficam registrados em cada programa.

## Versões da ferramenta

| Versão | Data | Destaques |
|---|---|---|
| 0.3.0 | 2026-09 | Repositório próprio; versão 2 da linguagem; CLI `new/test/explain/bundle/bench/upgrade`; playground em WebAssembly; instaladores para Linux e Windows; documentação em inglês, português e espanhol; testes diferenciais contra o motor da rede |
| 0.2.0 | 2026-09 | Versão 1 da linguagem dentro do repositório `thecoin` (The Coin v0.2.0) |

A lista completa de mudanças está no [CHANGELOG.md](https://github.com/LucasBolla94/tccl/blob/main/CHANGELOG.md). O plano para ativar a versão 2 na The Coin — incluindo ABI e estado versionados, combustível, contratos antigos, reexecução do histórico, testes, ativação e recuperação — está em [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

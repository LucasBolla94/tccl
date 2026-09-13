# Segurança

Esta página explica o que o motor garante, o que ele não consegue garantir e como escrever contratos que continuem seguros.

## Garantias do motor

São propriedades do compilador e da máquina virtual TCCL. Nenhum contrato, opção ou configuração as desliga.

| Garantia | Como |
|---|---|
| **Determinismo** | Sem relógio, ponto flutuante, aleatoriedade, threads ou iteração desordenada. A mesma chamada sobre o mesmo estado dá o mesmo resultado, combustível e armazenamento em todos os nós. O compilador faz parte do consenso: cada nó compila sozinho o código publicado. |
| **Aritmética verificada** | Toda operação com `int` é verificada. Estouro e divisão por zero fazem a chamada falhar; `mul_div` evita estouro intermediário. |
| **Combustível** | Toda instrução, expressão, acesso ao armazenamento, hash, assinatura e chamada custa combustível. Quando o combustível da transação acaba, a chamada falha. Um bloco tem limite de combustível, então o tempo de execução de um bloco é limitado. |
| **Limites de memória** | Valores têm no máximo 64 KiB, listas locais 4 096 itens e, na versão 2, todas as funções em execução juntas guardam no máximo 16 MiB. |
| **Compilação limitada** | Tamanho do código, aninhamento, profundidade de expressões e quantidades são limitados, então código hostil não esgota um nó durante a compilação. |
| **Tipos** | Todo valor tem um tipo; views não mudam nada; só funções payable recebem TCN; maps só existem no armazenamento. |
| **Transações atômicas** | Qualquer falha reverte toda mudança em todos os contratos envolvidos, toda transferência e todo evento. |
| **Sem reentrada** (v2) | Um contrato rodando numa transação não pode ser chamado de novo durante ela. |
| **Permissões explícitas** (v2) | As verificações `only` rodam antes do corpo da função e aparecem na interface. |
| **Transições** (v2) | Transições de enums são conferidas em toda gravação. |

## Falhas e taxas

Quando uma chamada falha — `require`, estouro, falta de combustível, transição não permitida, erro em outro contrato — **tudo o que a transação fez é revertido, e a taxa continua paga**. A rede fez o trabalho, e cobrar por ele é o que impede alguém de inundá-la com chamadas que falham.

- As carteiras simulam cada chamada sobre o estado atual e se recusam a enviar uma que falharia, então uma falha depois do envio normalmente significa que o estado mudou entre a simulação e o bloco (por exemplo, outra pessoa comprou o último item).
- Você paga pelo combustível que **reserva** (`max_fuel`), não só pelo usado. A carteira reserva o combustível medido × 1,3 + 5 000.
- Uma chamada que falha devolve o valor anexado e não altera depósitos.

Detalhes e números: [Combustível, taxas e depósitos](fees.md).

## Quem chama, origin e autorização

- Autorize com `caller` ou `only`. Numa chamada vinda de outro contrato, `caller` é esse contrato.
- Nunca autorize com `origin`: é quem assinou, mesmo com um contrato desconhecido no meio.
- Defina donos e papéis no `init()`; não deixe uma action de configuração desprotegida.

Veja [Permissões](permissions.md) e [Chamar outros contratos](calls.md).

## Dinheiro

- Pague a partir da **sua própria contabilidade**, não de `balance`: qualquer um pode aumentar o saldo de um contrato com uma transferência simples.
- Atualize o estado antes de enviar ou chamar (*verificações → efeitos → chamadas*).
- Liquide uma vez só: marque o pagamento como feito (um status enum com transições é ideal) antes de pagar.
- Prefira **pagamentos por saque** — os usuários retiram o que lhes é devido — em vez de percorrer uma lista de destinatários.
- Multiplique antes de dividir, ou use `mul_div(a, b, c)`. Rejeite valores negativos.

## Negação de serviço

- Nunca percorra uma lista que outras pessoas podem aumentar sem limite: cada volta custa combustível e a chamada acabará sempre falhando.
- Guarde dados por usuário em maps; percorra só listas limitadas.
- Lembre que armazenamento custa um depósito reembolsável pago por quem faz o contrato crescer.

## Upgrades

Um contrato atualizável só é tão confiável quanto sua **autoridade de upgrade**, que pode trocar o código — inclusive as verificações de permissão — na hora. Confira `is_final(addr)` antes de depender de outro contrato e torne o seu final quando estiver estável. Veja [Upgrades](upgrades.md).

## Privacidade e seus limites {#privacy}

Tudo o que fica numa blockchain pública — estado, argumentos, eventos, saldos — é visível para todos, para sempre. `only` controla quem pode *mudar* dados, não quem pode *ler*.

A TCCL oferece **assinaturas em anel vinculáveis** (`ring_verify`, bLSAG sobre Ristretto255, uma construção consolidada também usada pelo Monero) para pools de pagamentos privados como [`private_pool.tccl`](recipes.md#more-recipes). Um saque prova "sou dono de um destes N depósitos" sem dizer qual, e a key image impede que o mesmo depósito seja sacado duas vezes. Conheça os limites:

- **Conjunto de anonimato.** Você se esconde entre os membros do anel (no máximo 64). Poucos depósitos, ou anéis escolhidos sem cuidado, revelam muito.
- **Valores.** Os pools usam uma denominação fixa; qualquer outro valor chama atenção.
- **Horário e comportamento.** Depositar e sacar em sequência, ou em padrões incomuns, liga você às duas pontas.
- **Taxas.** Quem paga a taxa do saque fica visível; use um relayer, em quem você precisa confiar que não vai registrar seus dados.
- **Metadados.** Seu nó, endereço IP e o comportamento da sua carteira estão fora do controle do contrato.
- **Não é conhecimento zero.** Assinaturas em anel escondem *qual* membro assinou, não *que* um deles assinou. Não escondem valores nem a lógica do programa.
- **Nada de criptografia nova.** A TCCL não inventa esquemas criptográficos. Não crie o seu com `sha256` e `blake3`; peça uma primitiva revisada.

## Dados externos (oráculos) {#external-data}

Um contrato não consegue ler a internet. Preços, resultados e eventos do mundo real só entram por transações, então são exatamente tão confiáveis quanto quem os fornece.

- Aceite dados externos **assinados** por chaves conhecidas (`verify_ed25519`) e amarre a mensagem a `self`, ao valor e a uma altura ou nonce para que não possa ser reaproveitada em outro lugar.
- Prefira vários assinantes independentes e um limiar, e rejeite dados velhos usando `height`.
- Planeje o que acontece quando o fornecedor para ou mente: prazos, um árbitro, reembolsos.
- Um segredo revelado numa transação (por exemplo para liberar um pagamento com trava de hash) fica público assim que a transação entra no mempool.

## Aleatoriedade {#randomness}

**Nada numa blockchain é aleatório.** Altura do bloco, hashes de dados anteriores e conteúdo das transações são conhecidos antes ou podem ser influenciados por mineradores e usuários. Um sorteio que os usa pode ser previsto ou manipulado.

Use **commit–reveal** entre as partes que têm algo em jogo: cada uma se compromete com `sha256(segredo + escolha)` e depois revela; o contrato confere o compromisso e combina os segredos. Quem revela por último pode se recusar a revelar se fosse perder, então dê um prazo e faça a recusa custar a aposta. A receita [`coin_flip.tccl`](recipes.md#coin-flip-game) faz exatamente isso. Para muitos participantes, use um limiar de assinantes independentes ou uma fonte aleatória verificável assinada fora da blockchain, e documente a premissa de confiança.

## Front-running

As transações esperam num mempool público antes de serem mineradas. Qualquer um pode ver uma negociação pendente e tentar agir antes.

- Proteja negociações com limites como `min_out` (veja [`pool.tccl`](recipes.md#exchange-pool)).
- Amarre assinaturas a `self`, ao destinatário, aos valores e às taxas.
- Use commit–reveal em leilões e jogos.

## Problemas conhecidos na versão 1 da linguagem {#v1-findings}

Ao construir a versão 2, medimos o motor em uso na The Coin v0.2.0 e descobrimos que a versão 1 cobra **cópias de valores por bytes, não por alocações**. Um contrato com uma lista local de 4 000 textos curtos que chama `len(xs)` num laço copia a lista inteira a cada volta por cerca de 500 de combustível. Na máquina de referência (Intel Xeon E5-1620 v2) isso custa cerca de **1 400–2 100 ns por unidade de combustível em vez de ~20**, então um bloco cheio dessas chamadas poderia levar **um a dois minutos** para executar em vez de cerca de um segundo. Ler uma lista grande gravada num map tem o mesmo problema (~1 650 ns por unidade), e uma única transação pode manter cerca de 65 MB de valores.

- **A versão 2 corrige**: cópias e valores lidos do armazenamento pagam por alocação, `xs[i]` e `len(xs)` não copiam mais a lista e a memória é limitada a 16 MiB. As mesmas cargas medem 12–21 ns por unidade.
- **A versão 1 não pode ser alterada** sem mudança de consenso, porque blocos passados precisam ser reexecutados de forma idêntica. As opções de mitigação — ativar a versão 2 para novas publicações e reprecificar o combustível das chamadas da versão 1 a partir de uma altura de ativação — estão analisadas em [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

As medições podem ser reproduzidas com `cargo test -p tccl --release --test perf -- --ignored --nocapture`.

## Checklist antes de valor real {#checklist}

1. Toda action que move dinheiro ou muda permissões tem `only` ou um `require` sobre `caller`.
2. Donos e papéis são definidos no `init()`.
3. Todo `require` tem um cenário que o faz falhar, e todo caminho de sucesso tem um que passa.
4. Pagamentos são liquidados uma vez; o estado é atualizado antes de enviar ou chamar.
5. Nenhum laço sem limite sobre listas que outros podem aumentar.
6. Valores em motes; multiplicações antes das divisões.
7. Mensagens assinadas incluem `self`, valores, destinatários e uma altura ou nonce.
8. Aleatoriedade usa commit–reveal com prazos; dados externos são assinados e conferidos quanto à idade.
9. Dependências conferidas com `is_final` ou `code_hash`.
10. O contrato rodou na testnet, o código está publicado e alguém além do autor revisou.
11. A autoridade de upgrade é uma multisig ou um contrato de governança — ou o contrato é final.

## Relatar uma vulnerabilidade

Por favor, não abra uma issue pública para uma vulnerabilidade no compilador, na máquina virtual ou na biblioteca padrão. Siga o [SECURITY.md](https://github.com/LucasBolla94/tccl/blob/main/SECURITY.md).

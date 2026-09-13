# Referência de erros

> Esta página é gerada a partir do catálogo do compilador (`tccl docs errors`). As mesmas explicações aparecem no `tccl check`, no `tccl explain <código>` e no playground.

As *mensagens* de erro são sempre em inglês porque fazem parte do comportamento do compilador (e, na versão 1 da linguagem, do histórico da blockchain). O código, a explicação e a correção são traduzidos.

## Erros de compilação

Um erro de compilação aparece como `arquivo:linha:coluna`, com o código, uma correção sugerida e muitas vezes uma dica “você quis dizer”. Um contrato que não compila nunca é publicado; na rede, uma transação de publicação cujo código não compila é recusada pela carteira antes do envio, e paga a taxa se mesmo assim for minerada.

### C001 — Caractere de tabulação

Os blocos são definidos pela indentação e a TCCL só aceita espaços, para que o significado de uma linha nunca dependa do editor.

**Correção:** Troque as tabulações por 4 espaços.

Mensagens incluem: `tabs are not allowed`

### C002 — Indentação

Depois de uma linha que termina com ':' as próximas linhas precisam estar mais indentadas; o bloco termina quando a indentação volta a um nível anterior.

**Correção:** Indente o corpo de cada bloco com o mesmo número de espaços (normalmente 4); use 'pass' para um bloco vazio.

Mensagens incluem: `indentation does not match`, `unexpected indentation`, `an indented block`, `empty block`

### C003 — Texto não terminado

Um texto precisa começar e terminar com " na mesma linha. Escapes: \n \t \" \\.

**Correção:** Feche as aspas ou escape aspas internas como \".

Mensagens incluem: `unterminated`, `unknown escape`

### C004 — Literal inválido

Inteiros são decimais (com _ opcional) e cabem em 128 bits; bytes são 0x seguido de um número par de dígitos hexadecimais. Não há decimais: valores são inteiros em motes.

**Correção:** Escreva valores como inteiros, ex.: 5 * TCN ou 250_000_000.

Mensagens incluem: `invalid number literal`, `integer literal out of range`, `hex bytes literal`, `invalid hex literal`

### C005 — Estrutura do contrato

Um arquivo publicável começa com 'contract Nome' e declara ao menos um init, action ou view. Módulos podem vir depois em seções 'module nome'.

**Correção:** Coloque 'contract MeuContrato' na primeira linha de código.

Mensagens incluem: `'contract <Name>'`, `only one contract`, `a contract needs at least one`, `only contains modules`

### C006 — Nome desconhecido

Todo nome precisa ser declarado: um 'let' local, um parâmetro, uma const, uma variável de estado ou um valor de contexto (caller, value, balance, height, self, origin).

**Correção:** Confira a grafia ou declare, ex.: 'let total: int = 0'.

Mensagens incluem: `unknown name`, `unknown variable`

### C007 — Tipo desconhecido

Os tipos são int, bool, text, bytes, address, list[T], map[K, V] (só em estado) e os records, enums e interfaces que você declarar.

**Correção:** Use um tipo embutido ou declare o record/enum.

Mensagens incluem: `unknown type`

### C008 — Chamada de função

Chamadas precisam usar uma função embutida ou um 'fn' com a quantidade e os tipos certos de argumentos.

**Correção:** Confira o nome e os parâmetros na declaração.

Mensagens incluem: `unknown function`, `takes 0 argument`, `takes 1 argument`, `takes 2 argument`, `takes 3 argument`, `argument(s),`

### C009 — Tipos incompatíveis

A TCCL nunca converte tipos sozinha: uma operação ou atribuição precisa exatamente do tipo declarado. Condições precisam ser bool.

**Correção:** Converta explicitamente com to_text, to_bytes ou to_int, ou mude o tipo declarado.

Mensagens incluem: `expected int, found`, `expected bool, found`, `expected text, found`, `expected bytes, found`, `expected address, found`, `cannot add`, `arithmetic needs`, `cannot compare`, `ordering comparisons`, `need bool operands`, `compound assignment is not defined`, `expected a `

### C010 — Falta return

Uma função que declara '-> tipo' precisa terminar com 'return', ou com um if/else em que todo ramo retorna.

**Correção:** Adicione um 'return <valor>' no final.

Mensagens incluem: `must return a`, `must return on every path`, `does not return a value`

### C011 — Views só leem

Uma view é uma consulta gratuita que qualquer um roda sem transação, por isso não pode mudar estado, enviar TCN, emitir eventos, ler 'value' nem chamar actions.

**Correção:** Transforme em 'action' ou mova a mudança para uma action.

Mensagens incluem: `a view cannot`, `changes state, sends TCN or emits events`, `not available in a view`, `view cannot call the action`

### C012 — Nome já usado

Cada nome é declarado uma vez por contrato e não pode esconder nomes embutidos (caller, value, len, ...).

**Correção:** Escolha outro nome.

Mensagens incluem: `already declared`, `is reserved`, `reserved name`, `duplicate`

### C013 — Maps e listas de estado

Maps só existem como variáveis de estado e são usados item a item; listas e maps de estado não podem ser copiados ou atribuídos inteiros.

**Correção:** Use m[chave], m.has(chave), m.remove(chave), xs[i], xs.push(v), len(xs).

Mensagens incluem: `maps can only be used as state`, `map keys must be`, `cannot assign a whole`, `is a map;`, `is a state list`

### C014 — Constantes

Constantes são calculadas na compilação a partir de literais, outras constantes, aritmética, variantes de enum e address("..."), e nunca mudam.

**Correção:** Use uma variável de estado para valores que mudam.

Mensagens incluem: `is a constant`, `is not a constant`, `constant values can only`, `constant expression`, `constants must be`

### C015 — Receber TCN

Só funções marcadas 'payable' (actions e init) recebem TCN; uma chamada a outro contrato só pode anexar TCN com 'with value' se a interface a marcar como payable.

**Correção:** Adicione 'payable' ao cabeçalho: action buy() payable:

Mensagens incluem: `can be payable`, `not payable in its interface`, `'with value'`, `views cannot receive TCN`

### C016 — Entry points não são auxiliares

init, actions e views são chamadas por transações e consultas; o código só chama auxiliares 'fn' (e outros contratos por interfaces).

**Correção:** Mova a lógica comum para um 'fn' e chame dos dois lugares.

Mensagens incluem: `is an entry point`

### C017 — Instrução

Uma linha precisa fazer algo: atribuir, chamar função, controlar fluxo. Comparações não podem ser encadeadas (use 'and').

**Correção:** Guarde o resultado com 'let' ou remova a linha.

Mensagens incluem: `does nothing as a statement`, `outside of a loop`, `range() can only`, `chained comparisons`

### C018 — Records

Um record agrupa campos nomeados e tipados. Construa com todos os campos nomeados, ex.: Order(buyer: caller, amount: 5); leia com order.amount.

**Correção:** Informe cada campo uma vez com 'nome: valor'.

Mensagens incluem: `record`, `has no field`, `needs every field`, `named fields`, `contains itself`

### C019 — Enums e transições

Um enum lista situações nomeadas. Linhas como 'Open -> Paid, Cancelled' declaram as mudanças permitidas; qualquer outra falha ao gravar.

**Correção:** Use Enum.Variante e liste as próximas permitidas após '->'.

Mensagens incluem: `variant`, `'->'`

### C020 — Papéis e permissões

'only' restringe quem pode chamar uma action: membros de um papel (role admin; grant admin to x) ou o endereço de uma variável de estado.

**Correção:** Declare 'role nome' e conceda no init(): grant nome to caller.

Mensagens incluem: `'only`, `unknown role`, `is a role`

### C021 — Interfaces

Uma interface descreve actions e views de outro contrato. As chamadas a usam: Token(addr).transfer(to, 5). As assinaturas usam int, bool, text, bytes, address e listas.

**Correção:** Declare a função na interface exatamente como o outro contrato define.

Mensagens incluem: `interface`

### C022 — Módulos

Um módulo é código reutilizável compilado dentro do contrato ('use std.token' ou uma seção 'module nome'). Não tem endereço; seu estado só muda pelas próprias funções.

**Correção:** Chame as funções do módulo: token.mint(to, amount).

Mensagens incluem: `module`, `std.`

### C023 — Upgrades

Um upgrade mantém o armazenamento: variáveis de estado não podem ser removidas nem mudar de tipo, records mantêm os campos e enums mantêm as variantes na ordem.

**Correção:** Mantenha as variáveis antigas (pode deixar de usá-las) e adicione novas.

Mensagens incluem: `upgrade`, `was removed`, `storage slot`

### C024 — Limite atingido

O compilador limita tamanho, aninhamento e quantidades para que todo nó compile qualquer contrato de forma rápida e segura.

**Correção:** Divida expressões longas com 'let' e contratos grandes em módulos ou vários contratos.

Mensagens incluem: `too many`, `too large`, `too deep`, `too deeply`, `too long`, `nested too`

### C025 — Endereço literal

address("...") recebe um endereço bech32m válido da rede para a qual se compila (tc1 mainnet, tct1 testnet, tcr1 regtest).

**Correção:** Copie o endereço de novo e confira o prefixo.

Mensagens incluem: `address literal`, `address prefix`, `invalid address`

### C026 — Sintaxe

A linha não segue a gramática na posição marcada.

**Correção:** Compare com a referência da linguagem ou um exemplo.

Mensagens incluem: `expected`

## Erros de execução

Um erro de execução interrompe a chamada. **Todo efeito da transação é revertido** — o armazenamento de todos os contratos envolvidos, o TCN enviado, os eventos e o valor anexado à chamada. A taxa da transação continua paga, porque a rede fez o trabalho. As carteiras simulam as chamadas antes e se recusam a enviar uma chamada que falharia.

| Código | Erro | Significado | Correção |
|---|---|---|---|
| R001 | `out of fuel` | A chamada usou todo o combustível reservado pela transação. Tudo é revertido; a taxa continua paga. | Limite laços, evite percorrer listas que outros podem aumentar, ou reserve mais combustível. |
| R002 | `requirement failed: …` | Um 'require' foi falso; a chamada para e todo efeito é revertido. | Leia a mensagem: ela diz qual condição não foi atendida. |
| R003 | `integer overflow` | Estouro de inteiro ou divisão por zero; a aritmética da TCCL é sempre verificada. | Valide entradas com require antes de dividir; use mul_div para a × b ÷ c. |
| R004 | `division by zero` | Estouro de inteiro ou divisão por zero; a aritmética da TCCL é sempre verificada. | Valide entradas com require antes de dividir; use mul_div para a × b ÷ c. |
| R005 | `index I out of bounds (length N)` | Um índice foi negativo ou ≥ tamanho (também pop em lista vazia). | require i >= 0 and i < len(xs) |
| R006 | `value too large` | Um valor passou de 64 KiB, uma lista de 4 096 itens, o limite de eventos, ou as funções em execução usaram mais de 16 MiB. | Guarde dados em maps de estado em vez de listas locais grandes. |
| R007 | `call depth limit reached` | Mais de 16 chamadas de função aninhadas ou 8 contratos na pilha. | Troque recursão por laços; reduza cadeias de chamadas entre contratos. |
| R008 | `unknown function 'f'` | A função não existe, não é do tipo chamado (action/view) ou recebeu argumentos errados. | Confira a interface do contrato (tccl abi). |
| R009 | `function 'f' cannot be called this way` | A função não existe, não é do tipo chamado (action/view) ou recebeu argumentos errados. | Confira a interface do contrato (tccl abi). |
| R010 | `wrong arguments: …` | A função não existe, não é do tipo chamado (action/view) ou recebeu argumentos errados. | Confira a interface do contrato (tccl abi). |
| R011 | `function does not accept TCN (not payable)` | TCN foi enviado a uma função que não é 'payable'. O valor volta com a reversão. | Não envie valor, ou marque a action como payable. |
| R012 | `state cannot be modified in a view` | Uma view (ou contrato chamado por uma view) tentou mudar algo. | Use uma action. |
| R013 | `invalid amount` | send() precisa de valor positivo e saldo suficiente no contrato. | Controle no estado o que o contrato deve e confira antes de enviar. |
| R014 | `insufficient contract balance` | send() precisa de valor positivo e saldo suficiente no contrato. | Controle no estado o que o contrato deve e confira antes de enviar. |
| R015 | `contract cannot be destroyed while it still has storage (N entries)` | destroy() exige maps e listas vazios e não é permitido quando outro contrato chamou este. | Remova todos os itens antes e chame destroy direto de uma transação. |
| R016 | `host error: …` | A chamada falhou e toda mudança feita foi revertida. | Leia a mensagem para detalhes. |
| R017 | `internal type error: …` | A chamada falhou e toda mudança feita foi revertida. | Leia a mensagem para detalhes. |
| R018 | `re-entrant call: contract … is already running in this transaction` | Um contrato que já estava rodando nesta transação foi chamado de novo. A rede sempre bloqueia isso para impedir ataques de reentrada. | Desenhe fluxos em que as chamadas vão num sentido (A → B) e passe dados por argumentos ou retornos. |
| R019 | `contract call depth limit reached` | Mais de 16 chamadas de função aninhadas ou 8 contratos na pilha. | Troque recursão por laços; reduza cadeias de chamadas entre contratos. |
| R020 | `no contract at …` | Não há contrato no endereço, ou a função dele não corresponde à interface. | Confira o endereço e declare a interface igual à ABI do destino. |
| R021 | `interface mismatch: …` | Não há contrato no endereço, ou a função dele não corresponde à interface. | Confira o endereço e declare a interface igual à ABI do destino. |
| R022 | `memory limit reached (16777216 bytes)` | Um valor passou de 64 KiB, uma lista de 4 096 itens, o limite de eventos, ou as funções em execução usaram mais de 16 MiB. | Guarde dados em maps de estado em vez de listas locais grandes. |
| R023 | `transition not allowed: E cannot go from A to B` | O enum declara quais mudanças são permitidas e esta não está na lista. | Siga o caminho declarado (ex.: Paid → Shipped → Delivered) ou adicione a transição. |
| R024 | `destroy() is only allowed when the contract is called directly by a transaction` | destroy() exige maps e listas vazios e não é permitido quando outro contrato chamou este. | Remova todos os itens antes e chame destroy direto de uma transação. |
| R025 | `not supported: …` | A chamada falhou e toda mudança feita foi revertida. | Leia a mensagem para detalhes. |

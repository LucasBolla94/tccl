# Referência da linguagem

Esta página descreve a versão 2 da linguagem. O que está marcado **v2** é novo na versão 2; todo o resto também existe na versão 1 e se comporta igual lá.

## Estrutura do arquivo

```tccl
contract Shop                      # 1. cabeçalho: a primeira linha de código

use std.token                      # 2. módulos (v2)

const MAX_NAME: int = 64           # 3. declarações, em qualquer ordem
state owner: address
state prices: map[text, int]
event Sold(item: text, buyer: address, price: int)
role clerk                         # v2

init():                            # 4. funções
    owner = caller
    grant clerk to caller

action buy(item: text) payable:
    require prices.has(item), "unknown item"
    require value == prices[item], "wrong price"
    send(owner, value)
    emit Sold(item, caller, value)

view price_of(item: text) -> int:
    return prices[item]
```

- A primeira linha de código é `contract Nome`. Um arquivo guarda um contrato; seções `module` podem vir depois ([Módulos](modules.md)).
- Os blocos são definidos pela indentação **só com espaços** (tabulação é erro de compilação). Linhas dentro de `( )` e `[ ]` podem continuar na linha seguinte. Comentários começam com `#`.
- As declarações podem vir em qualquer ordem. Funções podem chamar auxiliares declarados depois. Uma constante só pode usar constantes declaradas acima dela.
- Um contrato precisa de pelo menos um `init`, `action` ou `view`.
- Um nome é declarado uma vez por contrato e não pode esconder uma variável local nem um nome embutido.

**Palavras-chave:** `contract const state event init action view fn payable let if elif else while for in break continue return require send emit destroy pass true false and or not`

**Palavras contextuais (v2):** `module use as record enum interface role only grant revoke to from with value upgrade`. Elas só são palavras-chave onde a gramática as espera, então contratos da versão 1 que as usam como nomes (por exemplo um parâmetro chamado `to`) continuam compilando.

**Nomes reservados:** `caller value balance height self TCN int bool text bytes address list map len sha256 blake3 to_bytes to_text to_int min max abs slice verify_ed25519 ring_verify address_of zero_address range`. As funções da v2 `mul_div isqrt pow code_hash is_contract is_final` e o valor de contexto `origin` não são reservados: se um contrato declarar um desses nomes, vale a declaração dele.

## Tipos

| Tipo | Padrão | Literal | Observações |
|---|---|---|---|
| `int` | `0` | `42`, `-7`, `1_000_000` | Inteiro com sinal de 128 bits, aritmética verificada |
| `bool` | `false` | `true`, `false` | |
| `text` | `""` | `"oi\n"` | UTF-8; `len` conta bytes; escapes `\n \t \" \\` |
| `bytes` | vazio | `0xdead_beef` | Número par de dígitos hexadecimais |
| `address` | endereço zero | `address("tc1…")` | 20 bytes; o prefixo precisa ser da rede |
| `list[T]` | `[]` | `[1, 2, 3]` | `T` é qualquer tipo menos map |
| `map[K, V]` | vazio | — | Só em variáveis de estado; `K` é escalar ou enum |
| record **v2** | todos os campos no padrão | `Order(buyer: caller, amount: 5)` | Campos nomeados e tipados |
| enum **v2** | primeira variante | `Status.Open` | Situações nomeadas, transições opcionais |
| interface **v2** | endereço zero | `Token(addr)` | Um endereço usado para chamar outro contrato |

Escalares (`int bool text bytes address`) podem ser comparados com `==` e `!=`, usados como chaves de map e receber valor inicial em `state`. Enums também podem ser comparados, usados como chaves e receber valor inicial. Records e listas não podem ser comparados com `==`.

Não há conversões implícitas nem decimais. Valores são inteiros em **motes**: 1 TCN = `TCN` = 100 000 000 motes. No código escreva `5 * TCN`; a notação `5tcn` é só para argumentos de linha de comando e de cenários.

## Constantes e estado

```tccl
const FEE_BP: int = 25                        # calculada na compilação
const TREASURY: address = address("tc1yfugpgq45fs8xe80x4jam2j2w2kqhk9qnp2umy")
state owner: address                          # padrão: endereço zero
state name: text = "Cloud Token"              # escalares e enums podem ter valor inicial constante
state phase: Phase = Phase.Open               # v2
state holders: list[address]                  # gravada item a item
state balances: map[address, int]
state orders: map[int, Order]                 # v2: records em maps
```

Constantes podem usar literais, constantes anteriores, operadores, variantes de enum e `address("…")`.

Listas e maps de estado não podem ser atribuídos nem copiados inteiros: trabalhe com os itens. **Valores padrão nunca são gravados:** atribuir `0`, `false`, `""`, bytes ou lista vazios, o endereço zero, a primeira variante de um enum ou um record com todos os campos no padrão apaga a entrada, então `m.has(k)` passa a ser `false` e o depósito de armazenamento é devolvido.

## Funções

| Tipo | Cabeçalho | Observações |
|---|---|---|
| `init` | `init(params) [payable]:` | Opcional, no máximo um; roda na publicação; não retorna valor |
| `action` | `action nome(params) [-> T] [payable] [only …]:` | Chamada por transações; pode retornar valor (`return_value` no recibo) |
| `view` | `view nome(params) -> T:` | Consulta gratuita e somente leitura; precisa retornar valor |
| `fn` | `fn nome(params) [-> T] [only …]:` | Auxiliar privado, chamado só pelo código |
| `upgrade` **v2** | `upgrade(params) [only …]:` | Roda uma vez quando um upgrade instala este código ([Upgrades](upgrades.md)) |

- A versão 2 aceita `payable` antes ou depois do tipo de retorno. A versão 1 exige `-> T payable`.
- Só `action` e `init` podem ser `payable`. Enviar TCN a qualquer outra função falha com `function does not accept TCN (not payable)`.
- Uma view não pode atribuir estado, usar `send`, `emit` ou `destroy`, ler `value`, chamar um `fn` que faça algo disso nem chamar uma action de outro contrato.
- Uma função com tipo de retorno precisa retornar em todos os caminhos: a última instrução é um `return`, ou um `if`/`else` em que todos os ramos terminam com `return`.
- Pontos de entrada não podem ser chamados pelo código. Recursão é permitida até profundidade 16.
- `only` lista papéis ou variáveis de estado do tipo address; veja [Permissões](permissions.md).

## Instruções

```tccl
let fee: int = amount / 100              # locais: tipo e valor são obrigatórios
balances[to] += amount - fee             # = += -= *=  (+= também junta text e bytes)
orders[id].status = Status.Paid          # v2: campos de record, em locais e no armazenamento
require caller == owner, "only the owner"
if amount > 100 * TCN:
    pass
elif amount > 0:
    queue.push(to)
else:
    require false, "amount must be positive"
for i in range(0, 3):                    # início … fim-1
    continue
for who in queue:                        # lista local ou de estado
    break
while len(queue) > 10:
    queue.pop()
send(to, fee)                            # paga com o saldo do contrato
emit Paid(to, fee)
grant clerk to who                       # v2
revoke clerk from who                    # v2
```

- `require cond[, mensagem]` interrompe a chamada e **reverte todo efeito** quando `cond` é falsa (`requirement failed: mensagem`; sem mensagem: `requirement at line N failed`).
- `send(to, amount)` falha com `invalid amount` (≤ 0) ou `insufficient contract balance`. Nunca executa código no destinatário.
- `emit Evento(args…)` — os argumentos precisam corresponder aos campos declarados. Eventos vão para o recibo.
- `destroy(to)` — só em actions: apaga o contrato e paga saldo e depósito a `to`. Todos os itens de listas e entradas de maps precisam ser removidos antes. Não é permitido quando outro contrato chamou este.
- `return [valor]`, `break`, `continue`, `pass`.
- Uma expressão sozinha numa linha precisa ser uma chamada de função, uma chamada a outro contrato ou `xs.pop()`.
- O índice de uma atribuição composta é avaliado uma vez: `m[next_id()] += 1` chama `next_id()` uma única vez.

## Operadores

| Precedência (menor → maior) | Operadores | Tipos |
|---|---|---|
| 1 | `or` | bool, curto-circuito |
| 2 | `and` | bool, curto-circuito |
| 3 | `not` | bool |
| 4 | `==` `!=` · `<` `<=` `>` `>=` | mesmo tipo escalar ou enum · int |
| 5 | `+` `-` | int; `+` também text + text, bytes + bytes |
| 6 | `*` `/` `%` | int |
| 7 | `-` unário | int |
| 8 | `x[i]` `x.campo` `x.m(…)` `f(…)` | |

Estouro falha com `integer overflow`. `/` e `%` por zero falham. `/` trunca em direção a zero e `%` tem o sinal do operando da esquerda (`-7 / 2 == -3`, `-7 % 2 == -1`). Comparações encadeadas (`a < b < c`) são rejeitadas. Condições precisam ser `bool`: não existe "verdadeiro por conveniência".

Um valor de interface pode ser comparado com um endereço: `Token(a) == b`.

## Listas e maps

| Operação | Lista local | Lista de estado |
|---|---|---|
| literal `[a, b]`, `[]` | ✓ | — |
| `xs[i]`, `xs[i] = v`, `xs[i] += v` | ✓ | ✓ |
| `xs.push(v)` | ✓ | ✓ |
| `xs.pop()` (instrução ou expressão) | — | ✓ |
| `len(xs)` / `xs.len()` | ✓ | ✓ |
| `for x in xs:` | ✓ | ✓ |
| atribuir ou copiar a lista inteira | ✓ | — |

Listas locais têm no máximo 4 096 itens e 65 536 bytes. Maps aceitam `m[k]` (o padrão se ausente), `m[k] = v`, `m[k] += v`, `m.has(k)` e `m.remove(k)`. Maps não podem ser percorridos: mantenha uma lista de chaves se precisar. Para mudar uma lista guardada num map, copie para uma local, altere e grave de volta.

## Records **v2** {#records}

Um record agrupa campos nomeados e tipados.

```tccl
record Order:
    buyer: address
    item: text
    price: int
    status: Status

state orders: map[int, Order]

action place(item: text) -> int:
    next_id += 1
    orders[next_id] = Order(buyer: caller, item: item, price: prices[item], status: Status.Placed)
    return next_id

view buyer_of(id: int) -> address:
    return orders[id].buyer
```

- Construa um record com **cada campo nomeado exatamente uma vez**: `Order(buyer: …, item: …, price: …, status: …)`. Campos faltando, desconhecidos ou repetidos são erro de compilação.
- Leia campos com `.`: `o.price`, `orders[id].buyer`. Atribua em locais e no armazenamento: `o.price = 5`, `orders[id].status = Status.Paid`, `orders[id].price += 1`.
- Um campo pode ter qualquer tipo menos map, inclusive outro record ou uma lista. Um record não pode conter a si mesmo.
- Records podem ser parâmetros, retornos, campos de eventos, variáveis de estado e valores de maps e listas. Não podem ser chaves de map nem comparados com `==`.
- Um record com todos os campos no padrão é o record padrão: gravá-lo apaga a entrada.
- Records declarados num módulo se chamam `modulo.Record` no contrato.
- Na ABI, argumentos são escritos `{buyer: tc1…, item: "lamp", price: 5, status: Placed}`.

## Enums e transições **v2** {#enums}

Um enum lista situações nomeadas. A primeira variante é o padrão.

```tccl
enum Color: Red, Green, Blue           # forma em linha, sem transições

enum Status:                           # forma em bloco com transições permitidas
    Placed -> Paid, Cancelled
    Paid -> Shipped, Refunded
    Shipped -> Delivered
    Delivered
    Cancelled
    Refunded
```

- Escreva variantes como `Status.Paid` (ou `modulo.Status.Paid`). `to_text(s)` devolve o nome da variante, por exemplo `"Paid"`.
- Enums podem ser comparados com `==`/`!=`, usados como chaves de map, constantes, valores iniciais de estado, campos de record e parâmetros (argumentos são escritos com o nome da variante: `Paid`).
- Se **qualquer** variante lista `-> próxima, …`, as transições valem para esse enum. Uma variante sem seta é final: nada pode vir depois dela.
- As transições são verificadas **sempre que um valor é gravado**: numa variável de estado, num valor de map, num item de lista de estado ou num campo de record gravado em qualquer um desses. Mudar `orders[id].status` de `Placed` para `Delivered` falha com `transition not allowed: Status cannot go from Placed to Delivered`, e a chamada inteira é revertida.
- Continuar na mesma variante é sempre permitido, então atualizar outros campos de um record não dispara a verificação.
- Uma entrada nova de map começa na variante padrão (a primeira); um `push` numa lista de estado também é verificado a partir do padrão.
- Variáveis locais não são verificadas: só importa o que é gravado.
- Um upgrade pode acrescentar variantes no final, nunca reordenar nem remover ([Upgrades](upgrades.md)).

## Papéis e `only` **v2**

```tccl
role manager
state treasurer: address

init():
    grant manager to caller

action set_price(item: text, price: int) only manager:
    prices[item] = price

action pay_out(to: address, amount: int) only manager, treasurer:
    send(to, amount)

view is_manager(who: address) -> bool:
    return manager.has(who)
```

- `role nome` declara um conjunto de endereços guardado pelo contrato.
- `grant nome to <endereço>` e `revoke nome from <endereço>` alteram o conjunto e emitem automaticamente `RoleGranted(role, account, by)` ou `RoleRevoked(role, account, by)`.
- `nome.has(endereço)` diz se um endereço tem o papel.
- `only a, b` numa action, `fn` ou `upgrade()` só deixa a chamada continuar se `caller` tiver o papel `a`, ou for o endereço guardado em `b` (variável de estado do tipo address), e assim por diante. Senão falha com `only a or b can call 'f'`.
- As regras aparecem na interface (`tccl check`, `tccl abi`, playground), para que os usuários vejam quem pode fazer o quê.

Detalhes e padrões: [Permissões](permissions.md).

## Interfaces e chamadas **v2**

```tccl
interface Token:
    action transfer(to: address, amount: int) -> bool
    action transfer_from(from: address, to: address, amount: int) -> bool
    view balance_of(who: address) -> int
    action buy() payable

action pay(token: address, to: address, amount: int):
    require Token(token).transfer(to, amount), "transfer failed"

action shop(t: address):
    Token(t).buy() with value 2 * TCN

view held(token: address) -> int:
    return Token(token).balance_of(self)
```

- `Nome(endereço)` trata um endereço como um contrato com essa interface; `let t: Token = Token(a)` o guarda.
- Dentro do contrato chamado, `caller` é **o contrato que chamou**; `origin` é a conta que assinou a transação.
- `with value <valor>` envia TCN do saldo do contrato que chama; a interface precisa marcar a função como `payable`.
- Assinaturas de interface usam `int bool text bytes address` e listas deles. Na execução, a função chamada precisa ter exatamente o mesmo tipo, os mesmos parâmetros e o mesmo retorno, senão a chamada falha com `interface mismatch`.
- Uma falha no contrato chamado reverte a transação inteira. Um contrato que já está rodando não pode ser chamado de novo (reentrada é sempre rejeitada).

Tudo sobre chamadas — identidade, transferências, atomicidade, limites — está em [Chamar outros contratos](calls.md).

## Valores de contexto

| Nome | Tipo | Significado |
|---|---|---|
| `caller` | `address` | Quem chamou esta função: quem assinou ou o contrato que chamou. Endereço zero dentro de views |
| `origin` **v2** | `address` | A conta que assinou a transação, mesmo dentro de uma chamada vinda de outro contrato |
| `value` | `int` | Motes enviados nesta chamada (indisponível em views) |
| `balance` | `int` | Saldo deste contrato em motes, incluindo `value` |
| `height` | `int` | Altura do bloco (cerca de um bloco por minuto) |
| `self` | `address` | O endereço deste contrato |
| `TCN` | `int` | 100 000 000 |

## Funções embutidas

| Função | Resultado | Combustível extra |
|---|---|---|
| `len(list \| text \| bytes)` | `int` | — |
| `min(a, b)`, `max(a, b)`, `abs(a)` | `int` | — |
| `to_text(int \| enum)` | texto decimal ou o nome da variante | — |
| `to_bytes(int \| address \| text \| bool \| bytes)` | `bytes` (int: 16 bytes big-endian; address: 20; bool: 1) | — |
| `to_int(bytes)` | `int` sem sinal big-endian de até 15 bytes | — |
| `slice(bytes, start, end)` | `bytes` | — |
| `sha256(bytes \| text)`, `blake3(bytes \| text)` | 32 bytes | 60 + 20 a cada 64 bytes |
| `verify_ed25519(pk, msg, sig)` | `bool` (false para entrada malformada) | 3 500 + 1 a cada 64 bytes |
| `ring_verify(ring: list[bytes], msg, sig, key_image)` | `bool` — assinatura em anel vinculável (bLSAG, Ristretto255), ≤ 64 chaves | 5 000 + 10 000 por chave |
| `address_of(pk: bytes)` | `address` de uma chave pública de 32 bytes | — |
| `zero_address()` | `address` | — |
| `address("tc1…")` | endereço literal em tempo de compilação | — |
| `mul_div(a, b, c)` **v2** | ⌊a × b ÷ c⌋ em direção a zero, sem estouro intermediário | 30 |
| `isqrt(x)` **v2** | ⌊√x⌋ para x ≥ 0 | 30 |
| `pow(base, exp)` **v2** | potência inteira verificada, exp ≥ 0 | 30 |
| `code_hash(addr)` **v2** | BLAKE3 do código compilado de um contrato, vazio se não for contrato | 250 |
| `is_contract(addr)` **v2** | `bool` | 250 |
| `is_final(addr)` **v2** | `true` se o contrato não tem autoridade de upgrade | 250 |

```tccl
let msg: bytes = blake3(to_bytes(self) + to_bytes(caller) + to_bytes(amount))
require verify_ed25519(pk, msg, sig), "bad signature"
let out: int = mul_div(amount_in * 9_970, reserve_out, reserve_in * 10_000 + amount_in * 9_970)
```

## Limites

| Limite | Valor |
|---|---|
| Código-fonte (um arquivo, incluindo seções de módulo) | 48 000 bytes |
| Programa compilado | 262 144 bytes |
| Combustível por transação · por bloco (padrão da mainnet) | 10 000 000 · 50 000 000 |
| Profundidade de chamadas de função | 16 |
| Contratos na pilha de chamadas **v2** | 8 |
| Memória usada pelas funções em execução **v2** | 16 MiB por transação |
| Aninhamento de blocos, parênteses, tipos | 32 |
| Operadores de uma mesma precedência numa expressão · profundidade de expressão | 64 · 128 |
| Funções · variáveis de estado · locais por função | 256 · 256 · 1 024 |
| Records · campos por record · enums · variantes · interfaces **v2** | 128 · 64 · 128 · 256 · 64 |
| Módulos por contrato **v2** | 16 |
| Valor text, bytes ou lista | 65 536 bytes |
| Itens numa lista local | 4 096 |
| Eventos por chamada · argumentos por chamada | 64 · 32 |
| Tamanho do anel | 64 |

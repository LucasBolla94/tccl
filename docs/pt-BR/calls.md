# Chamar outros contratos

A versão 2 da linguagem permite que um contrato chame actions e views de outros contratos. É isso que torna tokens utilizáveis por corretoras, jogos utilizáveis por marketplaces e pagamentos combináveis.

> **Situação na rede.** Chamadas entre contratos fazem parte da versão 2 da linguagem. Elas já rodam no `tccl`, nos cenários e no playground. A The Coin v0.2.0 roda a versão 1 e ainda não as executa; veja [Versões](versions.md).

## Declarar e chamar

```tccl
contract Payer

interface Token:
    action transfer(to: address, amount: int) -> bool
    action transfer_from(from: address, to: address, amount: int) -> bool
    view balance_of(who: address) -> int
    action deposit() payable

action pay(token: address, to: address, amount: int):
    require Token(token).transfer(to, amount), "transfer failed"

view held(token: address) -> int:
    return Token(token).balance_of(self)
```

- Uma `interface` lista as funções de que você precisa, com as assinaturas exatas. Ela não precisa listar todas as funções do outro contrato.
- `Token(endereço)` usa um endereço pela interface. Você pode guardá-lo: `let t: Token = Token(addr)`, ou declarar `state token: address` e escrever `Token(token)`.
- A chamada devolve o tipo declarado. Para ignorar o resultado, faça a chamada sozinha na linha.
- Assinaturas podem usar `int`, `bool`, `text`, `bytes`, `address` e listas deles. Records, enums e interfaces são locais de cada contrato: passe os campos.

## Quem está chamando: `caller` e `origin`

Quando um usuário chama `Pool.swap` e o pool chama `Token.transfer_from`:

| Dentro de | `caller` | `origin` | `self` |
|---|---|---|---|
| `Pool.swap` | o usuário | o usuário | o pool |
| `Token.transfer_from` | **o pool** | o usuário | o token |

**Autorize com `caller`, nunca com `origin`.** Se um token conferisse `origin`, qualquer contrato com que o usuário interagisse poderia mover os tokens dele. `origin` existe para registros e para recusar chamadas que não vêm diretamente de uma pessoa (`require caller == origin`), o que também bloqueia qualquer contrato, inclusive carteiras multisig — use raramente.

Como o token vê o pool como `caller`, os usuários primeiro **aprovam** o pool no token (`token.approve(pool, amount)`), e o pool move tokens com `transfer_from(usuário, self, amount)`.

## Enviar TCN numa chamada

```tccl
action buy(shop: address, price: int) payable:
    require value == price, "send the price"
    Shop(shop).deposit() with value price
```

- `with value <valor>` move motes **do saldo do contrato que chama** para o contrato chamado, antes de a função chamada rodar. O contrato chamado o vê como `value`.
- A interface precisa marcar a função como `payable`, e a função chamada precisa mesmo ser payable; senão a chamada falha com `function does not accept TCN (not payable)`.
- `send(to, amount)` é diferente: paga um endereço e **nunca executa código**, mesmo que o endereço seja um contrato.

## Valores de retorno

O `return` da função chamada volta com o tipo que a interface declara. Na execução, o motor confere se a função chamada tem o mesmo tipo (`action` ou `view`), os mesmos parâmetros e o mesmo retorno; uma diferença falha com `interface mismatch: 'f' is action (text) -> int in the called contract but (int) -> int in the interface`.

## Tudo ou nada

Uma transação é **atômica**. Se algo falha em qualquer lugar — um `require` num contrato três chamadas abaixo, um estouro, falta de combustível, uma transição não permitida — então:

- toda mudança de armazenamento em **todos** os contratos envolvidos é descartada;
- toda transferência de TCN, inclusive `with value` e o valor que o usuário anexou, é desfeita;
- nenhum evento é registrado;
- a taxa da transação continua paga, porque a rede a executou.

Não existe `try`/`catch` na versão 2: um contrato não pode engolir a falha de outro e continuar num estado pela metade. Desenhe os fluxos para que uma falha seja um "não" claro. As carteiras simulam as chamadas antes e não enviam chamadas que falhariam.

## Reentrada é impossível

Um contrato que já está rodando na transação **não pode ser chamado de novo** durante ela. Se `A` chama `B` e `B` tenta chamar `A`, a chamada falha com `re-entrant call` e tudo é revertido. Um contrato chamando a si mesmo por uma interface é rejeitado do mesmo jeito. Essa proteção faz parte do motor e não pode ser desligada.

Isso elimina o ataque clássico em que o chamado reentra num contrato atualizado pela metade. Mesmo assim, é boa prática atualizar seu próprio estado antes de chamar fora (*verificações → efeitos → chamadas*), porque deixa o código fácil de entender:

```tccl
action withdraw(token: address, amount: int):
    require deposits[caller] >= amount, "not enough"
    deposits[caller] -= amount                         # efeitos primeiro
    require Token(token).transfer(caller, amount), "transfer failed"   # depois a chamada
```

Por isso callbacks (um token avisando o contrato que o chamou) não são possíveis. Use padrões de consulta: o outro contrato expõe uma view ou action que você chama.

## Views e chamadas somente leitura

- Uma `view` pode chamar views de outros contratos. Não pode chamar actions — isso é erro de compilação.
- Tudo o que é alcançado a partir de uma view roda somente leitura: uma tentativa de mudar estado falha com `state cannot be modified in a view`.

## Conferir o que você chama

```tccl
init(token_address: address):
    require is_contract(token_address), "not a contract"
    require is_final(token_address), "the token must not be upgradeable"
    token = token_address
    token_code = code_hash(token_address)
```

- `is_contract(addr)` — se existe um contrato ali.
- `is_final(addr)` — o contrato não tem autoridade de upgrade, então seu código nunca pode mudar ([Upgrades](upgrades.md)).
- `code_hash(addr)` — BLAKE3 do código compilado; compare com o hash do código que você revisou.

## Limites e custos

| | |
|---|---|
| Contratos na pilha de chamadas | 8 (`contract call depth limit reached`) |
| Profundidade de chamadas de função, somando todos os contratos | 16 |
| Combustível | Um único orçamento para a transação inteira, compartilhado por todos os contratos |
| Memória | 16 MiB de valores somando todas as funções em execução |
| Custo de uma chamada | 700 de combustível + 1 a cada 32 bytes de argumentos + carga de 100 + 1 a cada 100 bytes de código |
| `destroy()` | Só quando o contrato é chamado diretamente por uma transação |
| Contratos da versão 1 da linguagem | Não podem ser chamados por outros contratos (`not supported: calls into language version 1 contracts`): foram escritos quando `caller` era sempre quem assinou |

## Exemplo: um pool de troca

A receita [`pool.tccl`](recipes.md#exchange-pool) é uma corretora de produto constante que guarda dois tokens. `add_liquidity` e `swap` atualizam as reservas e depois chamam `transfer_from`/`transfer` nos contratos dos tokens. Se um negociante não aprovou tokens suficientes, o `require` do token falha e o swap inteiro — inclusive a atualização das reservas — é revertido. O cenário dele testa exatamente isso.

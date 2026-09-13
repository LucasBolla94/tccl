# Módulos e biblioteca padrão

A versão 2 da linguagem separa duas ideias fáceis de confundir:

| | Módulo | Contrato publicado |
|---|---|---|
| O que é | Código-fonte reutilizado na compilação | Um programa que vive num endereço da blockchain |
| Escrito como | `module nome` (uma seção ou um módulo padrão) | `contract Nome` |
| Tem endereço, saldo, depósito | Não | Sim |
| Publicado ou atualizado sozinho | Nunca | Sim |
| Como usar | `use std.token`, depois `token.mint(...)` | Por uma `interface` e uma chamada |
| Onde fica o estado | Dentro do contrato que o usa | No armazenamento dele |
| Quem pode mudar o estado | Só as funções do próprio módulo | Só o próprio código |
| Custo | O código passa a fazer parte do contrato (tamanho, combustível de compilação) | Chamá-lo custa uma chamada entre contratos |

Resumindo: **um módulo é copiado para dentro do seu contrato; um contrato é chamado.** Escolha um módulo para reaproveitar código e outro contrato para compartilhar estado ou fundos com outras aplicações.

## Usar um módulo padrão

```tccl
contract CloudCoin
use std.token

role minter

init(supply: int):
    token.setup("Cloud Coin", "CLD", 8)
    grant minter to caller
    token.mint(caller, supply)

action mint(to: address, amount: int) only minter:
    token.mint(to, amount)
```

- `use std.token` compila o módulo de token padrão dentro de `CloudCoin`. `use std.token as coin` escolhe outro nome.
- As **funções auxiliares** do módulo são chamadas pelo nome dele: `token.mint(to, amount)`.
- As **actions e views** do módulo passam a fazer parte da interface do contrato: `CloudCoin` agora tem `transfer`, `approve`, `transfer_from`, `balance_of` e as demais. Conflito de nome com o contrato é erro de compilação.
- O **estado** do módulo fica gravado dentro do contrato com nomes qualificados (`token.balances`). O contrato pode lê-lo (`token.total_supply`), mas só as funções do módulo podem alterá-lo — assim as regras do módulo (saldos nunca negativos, oferta igual à soma dos saldos) continuam verdadeiras não importa o que o contrato faça.
- Records e enums do módulo se chamam `token.Record` no contrato.

## Seus próprios módulos

Coloque seções `module` no final do arquivo:

```tccl
contract App
use mathx

view average(a: int, b: int) -> int:
    return mathx.mean(a, b)

module mathx
fn mean(a: int, b: int) -> int:
    return (a + b) / 2
```

Ou deixe o módulo em `mathx.tccl` ao lado do contrato (começando com `module mathx`). `tccl check`, `tccl run` e `tccl test` o encontram sozinhos, e `tccl bundle app.tccl -o deploy.tccl` gera o arquivo único que você publica: uma publicação sempre leva um único arquivo de código.

Um módulo pode declarar constantes, records, enums, interfaces, eventos, estado, papéis, auxiliares `fn`, actions e views. Não pode declarar `init()` nem `upgrade()` (exporte um `fn setup(...)`) e não pode usar `use` com outros módulos. Toda seção `module` precisa ser usada.

## Módulos padrão são congelados

A biblioteca padrão vem dentro do compilador e é **congelada por versão da linguagem**: o `std.token` da versão 2 sempre compila para o mesmo código. O programa compilado registra o hash BLAKE3 de cada módulo que inclui (o `tccl abi` mostra). Melhorias vão para uma nova versão da linguagem.

## `std.token` — tokens fungíveis

Estado: `token_name`, `token_symbol`, `token_decimals`, `total_supply`, `balances: map[address, int]`, `allowances`.
Eventos: `Transfer(from, to, amount)` (emissão: a partir do endereço zero; queima: para ele), `Approval(owner, spender, amount)`.

| Exportado | Tipo | Comportamento |
|---|---|---|
| `transfer(to, amount) -> bool` | action | Move tokens de quem chama; `to` não pode ser o endereço zero |
| `approve(spender, amount) -> bool` | action | Define quanto `spender` pode mover de quem chama |
| `transfer_from(from, to, amount) -> bool` | action | Move tokens de `from` usando a autorização de quem chama |
| `balance_of(who)`, `allowance(owner, spender)`, `total_supply_of()`, `name()`, `symbol()`, `decimals()` | views | |

| Auxiliar | Use para |
|---|---|
| `token.setup(name, symbol, decimals)` | Uma vez, no `init()`; nome 1–64 bytes, símbolo 1–12, casas decimais 0–18 |
| `token.mint(to, amount)` | Criar tokens — você decide quem pode chamar (por exemplo `only minter`) |
| `token.burn(owner, amount)` | Destruir tokens |
| `token.move(from, to, amount)` | Regras de transferência próprias |

> As autorizações seguem o padrão comum e herdam sua corrida clássica: para mudar uma autorização de N para M, defina 0 primeiro e depois M.

## `std.items` — itens únicos

Para colecionáveis, objetos de jogo, ingressos e certificados. Cada item tem um `id`, um `owner`, um `kind` (1–32 bytes), `metadata` (≤ 1 024 bytes) e a altura `created`, guardados no record `items.Item`.
Eventos: `ItemCreated(id, owner, kind)`, `ItemTransferred(id, from, to)`, `ItemApproved(id, owner, spender)`, `ItemBurned(id, owner)`.

| Exportado | Tipo | Comportamento |
|---|---|---|
| `transfer_item(id, to)` | action | Só o dono; limpa a aprovação |
| `approve_item(id, spender)` | action | O dono deixa um endereço mover o item |
| `take_item(id, to)` | action | O endereço aprovado move o item |
| `item(id) -> items.Item`, `owner_of(id)`, `items_owned_by(who)`, `item_count()` | views | |

Auxiliares: `items.create(to, kind, metadata) -> int`, `items.move_item(id, to)`, `items.burn_item(id)`. Os metadados são públicos; guarde um hash ou um link se o conteúdo for grande ou privado.

## `std.payments` — pagamentos condicionais

Guarda TCN até uma condição ser cumprida. Cada pagamento é um record `payments.Payment` com um `payments.Status` que só pode ir de `Pending → Released` ou `Pending → Refunded`.
Eventos: `PaymentCreated(id, payer, payee, amount)`, `PaymentReleased(id, payee, amount, reason)`, `PaymentRefunded(id, payer, amount, reason)`.

| Exportado | Quem | Condição |
|---|---|---|
| `create_payment(payee, arbiter, release_after, refund_after, hashlock) payable -> int` | qualquer um | Trava o TCN enviado. Use o endereço zero, `0` ou bytes vazios para desativar árbitro, alturas ou trava de hash |
| `release_payment(id)` | pagador ou árbitro | Paga o recebedor |
| `claim_payment(id)` | recebedor | Depois de `release_after` (se definido) |
| `reveal_payment(id, secret)` | qualquer um | Se `sha256(secret) == hashlock`: paga o recebedor (pagamento com trava de hash) |
| `refund_payment(id)` | recebedor ou árbitro a qualquer momento; pagador depois de `refund_after` | Devolve o TCN ao pagador |
| `payment(id)`, `payment_status(id)`, `locked_total()` | views | |

Auxiliares: `payments.create(...)`, `payments.pay_out(id, reason)`, `payments.pay_back(id, reason)`.

> Um segredo revelado fica público assim que a transação entra no mempool. Travas de hash protegem *quem recebe*, não o sigilo do segredo. Veja [Segurança](security.md#external-data).

Veja as [receitas](recipes.md) com contratos completos e testados usando cada módulo.

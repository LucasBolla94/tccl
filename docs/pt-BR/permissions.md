# Permissões

Quem pode fazer o quê é a pergunta mais importante de qualquer contrato. A versão 2 da linguagem deixa a resposta explícita no código e visível na interface.

## Papéis

```tccl
role manager
role shipper

init():
    grant manager to caller

action hire_shipper(who: address) only manager:
    grant shipper to who

action fire_shipper(who: address) only manager:
    revoke shipper from who

action ship(id: int) only shipper:
    orders[id].status = Status.Shipped
```

- `role nome` declara um conjunto de endereços. Um contrato pode declarar vários papéis; um módulo pode declarar os seus.
- `grant nome to <endereço>` adiciona um endereço; `revoke nome from <endereço>` remove. As duas são instruções e emitem `RoleGranted(role, account, by)` ou `RoleRevoked(role, account, by)`, então o histórico de permissões fica nos recibos.
- `nome.has(endereço)` diz se o endereço tem o papel.
- Um papel não é um valor: não pode ser atribuído, passado nem retornado.
- Ninguém tem um papel até o contrato concedê-lo. Conceda o primeiro administrador no `init()`.

## `only`

```tccl
state owner: address
state treasurer: address
role auditor

action withdraw(to: address, amount: int) only owner, treasurer:
    send(to, amount)

fn audit_log(note: text) only auditor:
    emit Audited(caller, note)
```

- `only a, b, …` vai no fim do cabeçalho da função. A chamada continua se `caller` tiver algum dos papéis listados ou for igual a alguma **variável de estado do tipo address** listada. Senão falha com `requirement failed: only owner or treasurer can call 'withdraw'` e nada muda.
- Funciona em actions, auxiliares `fn` e `upgrade()`. Não é permitido em views (consultas gratuitas não têm quem chama verificado) nem no `init()` (nenhum papel existe ainda).
- A verificação roda antes da primeira instrução da função, então nenhum trabalho é feito para quem não tem permissão.
- `tccl check`, `tccl abi` e o playground mostram as regras ao lado de cada função, por exemplo `action withdraw(to: address, amount: int) only owner, treasurer`.

`only owner` é exatamente `require caller == owner, "only owner can call '…'"` escrito de um jeito que as ferramentas conseguem ler. Contratos da versão 1 usam a forma com `require`.

## Padrões

**Transferência de dono**

```tccl
state owner: address
state pending_owner: address

action offer_ownership(to: address) only owner:
    pending_owner = to

action accept_ownership() only pending_owner:
    owner = pending_owner
    pending_owner = zero_address()
```

As duas etapas evitam entregar o contrato a um endereço digitado errado.

**Nunca se tranque para fora.** Revogar o último administrador é permanente. Mantenha uma contagem se isso importar:

```tccl
role admin
state admins: int

action add_admin(who: address) only admin:
    require not admin.has(who), "already an admin"
    grant admin to who
    admins += 1

action remove_admin(who: address) only admin:
    require admin.has(who), "not an admin"
    require admins > 1, "keep at least one admin"
    revoke admin from who
    admins -= 1
```

**Papéis para contratos.** Um papel pode ser concedido a um endereço de contrato. Numa chamada vinda desse contrato, `caller` é o contrato, então `only operator` funciona para contratos também — por exemplo um jogo autorizado a criar itens.

**Várias aprovações.** Para decisões que exigem M de N pessoas, guarde as aprovações por proposta e execute quando a contagem chegar a M — veja [`treasury.tccl`](recipes.md#more-recipes).

## Erros a evitar

- **Autorizar com `origin`.** Use sempre `caller` (ou `only`). `origin` é quem assinou a transação mesmo com um contrato desconhecido no meio.
- **Inicialização desprotegida.** Defina donos e conceda papéis no `init()`, que roda uma vez na publicação. Uma action `setup` que qualquer um pode chamar primeiro é uma tomada de controle.
- **Esquecer que views são públicas.** `only` não esconde dados: tudo o que fica gravado na blockchain pode ser lido por qualquer pessoa.
- **Confiar cegamente na autoridade de upgrade.** A autoridade de um contrato atualizável pode trocar o código, inclusive as verificações de permissão. Veja [Upgrades](upgrades.md).

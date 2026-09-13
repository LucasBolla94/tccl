# Receitas testadas

Cada receita é um contrato completo da pasta [`examples/`](https://github.com/LucasBolla94/tccl/tree/main/examples) com um cenário que confere o comportamento, inclusive as falhas. Todos os cenários rodam na integração contínua (`cargo test` e `tccl test examples`), então o código desta página funciona com esta versão.

Rode você mesmo:

```sh
git clone https://github.com/LucasBolla94/tccl && cd tccl/examples
tccl test counter.scenario
```

Ou abra o contrato no [playground](/pt-BR/playground/): o cenário aparece na aba *Cenário*.

## Contador

Estado, actions, views e eventos — o menor contrato útil. Versão 1 da linguagem, publicável na The Coin hoje.

{{example:counter.tccl}}

{{scenario:counter.scenario}}

## Pote de gorjetas

Uma action payable, saque só do dono e uma view que devolve vários números. Versão 1.

{{example:tip_jar.tccl}}

{{scenario:tip_jar.scenario}}

## Token com papel de emissor

O módulo de token padrão mais um papel. `transfer`, `approve`, `transfer_from` e as views vêm do `std.token`; o contrato só decide quem pode emitir.

{{example:cloud_coin.tccl}}

{{scenario:cloud_coin.scenario}}

## Pedidos com records, transições e papéis

Um record `Order` tipado, um enum `Status` cujas transições permitidas impedem mudanças impossíveis (um pedido enviado não pode ser cancelado) e dois papéis.

{{example:orders.tccl}}

{{scenario:orders.scenario}}

## Pool de troca {#exchange-pool}

Uma corretora de produto constante (DEX) para dois contratos de token, com taxa de 0,30 %. Mostra interfaces, `caller` dentro de outro contrato (os negociantes aprovam o pool), `mul_div` e `isqrt`, proteção contra variação de preço com `min_out` e atomicidade: um swap sem autorização suficiente falha no token e reverte a atualização das reservas do pool.

{{example:pool.tccl}}

{{scenario:pool.scenario}}

> Corretoras reais também precisam de proteção contra manipulação de preço dentro de um bloco quando outros contratos usam o preço do pool. Não use `quote` como oráculo de preço.

## Jogo de cara ou coroa {#coin-flip-game}

Um jogo para dois jogadores sem aleatoriedade falsa: o anfitrião se compromete com uma escolha escondida, o convidado tenta adivinhar e o anfitrião revela. Um anfitrião que se recusa a revelar perde depois de 20 blocos. Veja [Aleatoriedade](security.md#randomness).

{{example:coin_flip.tccl}}

{{scenario:coin_flip.scenario}}

## Ingressos como itens únicos

Ingressos emitidos por um organizador com `std.items`, capacidade limitada, transferências e check-in (queimando o ingresso).

{{example:tickets.tccl}}

{{scenario:tickets.scenario}}

## Pagamentos condicionais

Todas as actions vêm do `std.payments`: um pagamento com trava de hash liberado por quem revelar o segredo, e um pagamento que o comprador pode recuperar depois de um prazo.

{{example:deals.tccl}}

{{scenario:deals.scenario}}

## Atualizar um contrato {#upgrading-a-contract}

O contador acima, atualizado para uma segunda versão pela sua autoridade de upgrade. A nova variável de estado é inicializada por `upgrade()`, os valores antigos são mantidos, estranhos não conseguem atualizar e um contrato final nunca mais muda.

{{example:counter_v2.tccl}}

{{scenario:upgrade.scenario}}

## Escrow com árbitro

Papéis expressos com `require`, prazos, uma disputa e `destroy`. Versão 1.

{{example:escrow.tccl}}

{{scenario:escrow.scenario}}

## Mais receitas {#more-recipes}

Estes contratos da versão 1 são testados em `crates/tccl/tests/examples.rs`:

| Contrato | Mostra |
|---|---|
| [`shop.tccl`](../../examples/shop.tccl) | Todo tipo de declaração, payable, auxiliares |
| [`token.tccl`](../../examples/token.tccl) | Um token escrito à mão: maps, autorizações, chaves compostas |
| [`crowdfund.tccl`](../../examples/crowdfund.tccl) | Prazos e reembolsos |
| [`poll.tccl`](../../examples/poll.tccl) | Argumentos lista, listas de estado, laços limitados |
| [`savings.tccl`](../../examples/savings.tccl) | Travas de tempo e reembolso de armazenamento |
| [`treasury.tccl`](../../examples/treasury.tccl) | Aprovações M de N |
| [`names.tccl`](../../examples/names.tccl) | Chaves de texto, validação, expiração |
| [`private_pool.tccl`](../../examples/private_pool.tccl) | Assinaturas em anel para pagamentos privados (veja [Privacidade](security.md#privacy)) |

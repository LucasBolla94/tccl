# Seu primeiro contrato

Este tutorial leva cerca de dez minutos. Você vai instalar o `tccl`, criar um contrato, encontrar e corrigir um erro, testá-lo com um cenário, simular transações e aprender a publicá-lo na The Coin.

> Tudo dos passos 1 a 6 acontece no seu computador. Nada é assinado nem publicado, e nenhuma carteira ou frase de recuperação é usada.

## 1. Instalar

**Linux**

```sh
curl -fsSL https://tccl.the-coin.cloud/install.sh | sh
```

**Windows** (PowerShell)

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://tccl.the-coin.cloud/install.ps1 | iex"
```

Os dois scripts baixam a versão mais recente do GitHub e conferem o checksum SHA-256. Você também pode usar o instalador MSI ou compilar a partir do código — veja [Ferramentas](tools.md#install). Confira a instalação:

```sh
tccl version
```

Prefere não instalar nada? Abra o [playground](/pt-BR/playground/): ele roda o mesmo compilador e a mesma máquina virtual no seu navegador.

## 2. Criar um projeto

```sh
tccl new tip-counter
cd tip-counter
```

Isso cria dois arquivos: `contract.tccl` e `contract.scenario`. Abra `contract.tccl`:

```tccl
# TipCounter: a starting point. Edit it, then run `tccl check contract.tccl` and `tccl test`.
contract TipCounter

state owner: address
state count: int

event Increased(by: address, total: int)

init():
    owner = caller

action increment(amount: int):
    require amount > 0, "amount must be positive"
    count += amount
    emit Increased(caller, count)

action reset() only owner:
    count = 0

view get() -> int:
    return count
```

Leia de cima para baixo:

- Linhas que começam com `#` são comentários.
- `contract TipCounter` dá nome ao contrato. Um arquivo guarda um contrato.
- Variáveis `state` ficam gravadas na blockchain. `owner` começa como o endereço zero e `count` como `0`.
- `event Increased(...)` declara algo que o contrato pode anunciar. Eventos vão para o recibo da transação, onde carteiras e exploradores os leem.
- `init()` roda uma vez, na publicação. `caller` é quem enviou a transação, então quem publica vira o dono.
- Uma `action` é chamada por uma transação e pode mudar o estado. `require` interrompe a chamada — e reverte tudo — quando a condição é falsa.
- `only owner` deixa só o endereço guardado em `owner` chamar `reset`.
- Uma `view` é uma consulta gratuita e somente leitura. Ela precisa retornar um valor.

Palavras-chave e mensagens do contrato ficam em inglês, como em toda linguagem de programação; nomes e textos são seus.

## 3. Verificar

```sh
tccl check contract.tccl
```

```text
✔ TipCounter compiles (language 2, 438 bytes of source, 376 bytes compiled)
  state: owner: address, count: int
  init init()
  action increment(amount: int)
  action reset() only owner
  view get() -> int
  can: change state, emit events
```

O `tccl check` compila exatamente como a rede e mostra a interface: cada ponto de entrada, quem pode chamá-lo e o que o contrato pode fazer.

## 4. Quebrar de propósito

Troque `return count` por `return cout` e verifique de novo, pedindo explicações em português:

```sh
tccl check contract.tccl --lang pt
```

```text
error[C006]: unknown name 'cout'
  --> contract.tccl:21:12
   |
21 |     return cout
   |            ^
  = por quê: Todo nome precisa ser declarado: um 'let' local, um parâmetro, uma const, uma variável de estado ou um valor de contexto (caller, value, balance, height, self, origin).
  = correção: Confira a grafia ou declare, ex.: 'let total: int = 0'. (did you mean 'count'?)
```

Todo erro tem posição, um código que você consulta com `tccl explain C006 --lang pt` ou na [referência de erros](errors.md), uma explicação e uma correção. A mensagem em si fica em inglês porque faz parte do comportamento do compilador. Desfaça a mudança antes de continuar.

## 5. Testar com um cenário

`contract.scenario` é um pequeno roteiro de teste:

```scenario contract.scenario
deploy contract.tccl as app --from owner
call app increment 5 --from bob
expect ok
expect event Increased
view app get
expect result 5
call app reset --from bob
expect fail "only owner"
call app reset --from owner
expect ok
```

Rode todos os cenários da pasta:

```sh
tccl test
```

```text
✔ ./contract.scenario (5 checks)
5 checks passed, 0 failed, 1 scenario file(s)
```

Contas como `owner` e `bob` são fictícias; cada uma começa com 1 000 000 TCN. Cenários são o jeito mais rápido de provar que permissões e falhas se comportam como você quer. A sintaxe completa está em [Ferramentas](tools.md#scenarios).

## 6. Simular transações

O `tccl run` mantém uma blockchain simulada em `tccl-state.json`, para explorar passo a passo:

```sh
tccl run contract.tccl deploy
tccl run contract.tccl call increment 5 --from bob
tccl run contract.tccl call reset --from bob
tccl run contract.tccl view get
tccl run contract.tccl state
```

```text
event Increased(by: tcr14vqs008pe0yx022r6pneffl2mnn0x2lkzh7452, total: 5)
ok · fuel used: 1152 · height: 3 · bob balance: 100000000000000 motes
estimated on-chain cost (default parameters): fee 9247 motes for max_fuel 6497 and ~175 bytes · storage deposit +0 motes
FAILED[R002]: requirement failed: only owner can call 'reset' · fuel used: 281 (all changes reverted)
```

O simulador mostra o combustível usado e uma **estimativa** da taxa e do depósito de armazenamento com os parâmetros padrão da The Coin. Os endereços começam com `tcr1` porque contas simuladas não são contas da mainnet (`tc1`) nem da testnet (`tct1`).

## 7. Publicar na The Coin

A publicação usa a carteira do software do nó, o [`thecoin-wallet`](https://the-coin.cloud/docs.html), e não o `tccl`. O `tccl` nunca lida com chaves.

> **Versão da linguagem na rede.** A The Coin v0.2.0 roda a versão 1 da linguagem. O modelo acima usa `only`, que é da versão 2. Para publicar hoje, troque `only owner` por `require caller == owner, "only the owner"` e verifique com `tccl check --language 1 contract.tccl`.

Comece sempre pela testnet:

```sh
thecoin-wallet --network testnet contract deploy contract.tccl
thecoin-wallet --network testnet contract invoke <endereço> increment 5
thecoin-wallet --network testnet contract view <endereço> get
```

A carteira simula cada chamada antes de enviar e se recusa a mandar chamadas que falhariam. Veja [Ferramentas](tools.md#testnet) e [Combustível, taxas e depósitos](fees.md).

## Próximos passos

- Mexa nos exemplos do [playground](/pt-BR/playground/) — experimente `orders.tccl` e `pool.tccl`.
- Leia a [referência da linguagem](language.md).
- Antes de lidar com valor real, leia [Segurança](security.md) e escreva um cenário para cada `require`.

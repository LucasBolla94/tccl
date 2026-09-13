# Upgrades

Às vezes o código publicado precisa de correções. A TCCL usa uma **autoridade de upgrade**: cada contrato tem no máximo um endereço autorizado a trocar seu código, e esse endereço pode passar o poder adiante ou renunciar a ele para sempre. Nada é atualizado implicitamente — todo upgrade é uma transação assinada pela autoridade.

> **Situação na rede.** Upgrades são definidos pela versão 2 da linguagem e funcionam no `tccl`, nos cenários e no playground. A The Coin v0.2.0 não tem transação de upgrade: contratos publicados lá hoje não podem mudar. Ativar upgrades exige uma mudança de protocolo descrita em [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

## O modelo

| | |
|---|---|
| Na publicação | Quem publica vira a autoridade de upgrade, a menos que o contrato seja publicado como **final** (`--final`) |
| Atualizar | A autoridade envia o novo código-fonte; ele substitui o código **na hora** se for compatível |
| Trocar a autoridade | A autoridade pode passá-la a outro endereço, por exemplo uma multisig ou um contrato de governança |
| Renunciar | A autoridade pode defini-la como nenhuma: o contrato fica **final** para sempre |
| Contratos finais | Nunca mais podem ser atualizados |

Isso é flexível — uma equipe pode corrigir um bug no mesmo dia — e essa flexibilidade também é o risco: **quem tem a autoridade controla o contrato**, inclusive fundos e permissões. Usuários devem tratar um contrato atualizável como alguém que confia na sua autoridade.

## O que usuários e outros contratos podem conferir

- `tccl run … state` e o playground mostram a autoridade de upgrade e a versão do código.
- Outros contratos podem exigir dependências imutáveis: `require is_final(token), "token must be final"`.
- `code_hash(addr)` identifica o código exato; compare com o hash do código que você revisou.

Boas práticas para projetos que mantêm uma autoridade:

- guarde-a numa multisig ou num contrato de governança, não numa única chave online;
- anuncie upgrades e publique o novo código antes de enviá-los;
- torne o contrato final quando ele estiver estável.

## Regras de compatibilidade

O armazenamento sobrevive ao upgrade, então o novo código precisa ler os dados antigos do mesmo jeito. O `tccl` confere, antes de qualquer execução:

- toda variável de estado do código antigo continua existindo **com o mesmo nome e um tipo compatível** — não dá para remover nem mudar o tipo de uma variável (pare de usá-la);
- novas variáveis de estado entram depois das existentes (o compilador mantém cada variável existente na sua posição de armazenamento automaticamente, em qualquer ordem que você as escreva);
- records gravados no estado mantêm **exatamente os mesmos campos** na mesma ordem;
- enums gravados no estado mantêm as variantes na ordem; novas variantes só podem ser **acrescentadas no final**;
- a versão da linguagem não volta para trás.

Todo o resto — funções, eventos, constantes, regras de papéis, novos records — pode mudar. O relatório de upgrade lista estado adicionado, funções adicionadas, removidas e alteradas, e observações como *"the new code can send TCN (the old code could not)"*.

## `upgrade()` — migrar dados

```tccl
contract Counter

state count: int
state last_caller: address
state step: int                      # novo nesta versão

upgrade():
    step = 10                        # roda uma vez, dentro da transação de upgrade

action increment(amount: int):
    count += amount * step
```

- `upgrade(params)` roda uma vez quando o upgrade instala o código, chamado pela autoridade (`caller`). Pode receber parâmetros e usar `only`.
- Se falhar, o upgrade inteiro é revertido e o código antigo continua.
- Não pode ser chamado depois: não é uma action.

## Na prática

```sh
tccl run counter.tccl deploy --from dev               # dev é a autoridade de upgrade
tccl run counter_v2.tccl upgrade --from dev
tccl run counter.tccl authority @multisig --from dev  # passa a autoridade
tccl run counter.tccl authority none --from multisig  # torna o contrato final
```

Num cenário:

```scenario upgrade.scenario
deploy counter.tccl as counter --from dev
upgrade counter counter_v2.tccl --from bob
expect fail
upgrade counter counter_v2.tccl --from dev
expect ok
authority counter none --from dev
```

A receita completa e testada é [Atualizar um contrato](recipes.md#upgrading-a-contract).

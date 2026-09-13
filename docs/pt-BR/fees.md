# Combustível, taxas e depósitos

## Combustível

Toda operação custa **combustível**. Uma transação reserva um máximo (`max_fuel`); se a execução precisar de mais, a chamada falha e tudo é revertido. Os preços são calibrados para que cerca de 20 ns de CPU num servidor de 2 vCPUs correspondam a uma unidade de combustível, o que mantém um bloco cheio (50 000 000 de combustível na mainnet, por padrão) em cerca de um segundo no pior caso.

| Operação | Combustível |
|---|---|
| Instrução · expressão | 2 · 1 |
| A cada 32 bytes de valores lidos, copiados ou combinados | 1 |
| Chamada de função | 20 |
| Leitura do armazenamento | 250 |
| Escrita ou remoção no armazenamento | 400 + 4 por byte de chave e valor |
| `send` · `emit` · `destroy` | 300 · 100 + 1 por byte · 1 000 |
| `sha256`, `blake3` | 60 + 20 a cada 64 bytes |
| `verify_ed25519` | 3 500 + 1 a cada 64 bytes |
| `ring_verify` | 5 000 + 10 000 por chave |
| Publicação: compilar | 5 por byte de código-fonte |
| Chamada: carregar o contrato | 100 + 1 a cada 100 bytes de código compilado |

### Mudanças na versão 2 da linguagem

| Operação | Versão 1 | Versão 2 |
|---|---|---|
| Copiar um valor (ler uma local, uma constante) | 1 a cada 32 bytes | 1 a cada 32 bytes **+ 4 por alocação** (cada text, bytes ou lista dentro dele) |
| Decodificar um valor lido do armazenamento | incluído na leitura | **+ 1 a cada 32 bytes + 6 por alocação** |
| `xs[i]` e `len(xs)` numa lista local | copia a lista inteira | lê sem copiar |
| Chamada a outro contrato | — | 700 + 1 a cada 32 bytes de argumentos + carga (100 + 1 a cada 100 bytes de código) |
| `mul_div`, `isqrt`, `pow` | — | 30 |
| `code_hash`, `is_contract`, `is_final` | — | 250 |
| Verificação de transição de enum | — | 1 por nível verificado |

Código da versão 1 compilado como versão 2 produz os mesmos resultados, armazenamento e eventos; o combustível só pode mudar onde há cópias de listas ou valores grandes. Os motivos estão em [Segurança](security.md#v1-findings).

## Taxas

A taxa mínima de uma transação é

```text
taxa = (base_fee + ⌈bytes × fee_per_kb ÷ 1000⌉ + ⌈max_fuel × fee_per_kfuel ÷ 1000⌉) × congestionamento
```

com os parâmetros padrão da The Coin `base_fee = 1 000`, `fee_per_kb = 10 000` e `fee_per_kfuel = 1 000` motes. O multiplicador de congestionamento começa em 1× e se ajusta com o uso dos blocos; a parte acima de 1× é queimada. As prioridades da carteira `low | normal | high | urgent` pagam 1×, 1,25×, ≥ 2× e ≥ 4× o mínimo. Os parâmetros são definidos pela governança, então confira os valores atuais da rede antes de depender de números exatos.

- Você paga pelo combustível que **reserva**. A carteira reserva o combustível medido × 1,3 + 5 000.
- Uma transação que **falha** paga a taxa; todos os seus efeitos são revertidos.
- Views chamadas pela API são gratuitas e não criam transações.

O `tccl run` e o playground mostram uma estimativa com os parâmetros padrão, por exemplo:

```text
estimated on-chain cost (default parameters): fee 9247 motes for max_fuel 6497 and ~175 bytes · storage deposit +0 motes
```

## Depósitos de armazenamento

O estado de um contrato é garantido por um **depósito reembolsável**:

- O tamanho de um contrato é o código compilado mais cada entrada de armazenamento (chave + valor).
- Depósito exigido = ⌈tamanho ÷ 1 000⌉ × `storage_deposit_per_kb` (padrão 100 000 motes, 0,001 TCN por kB iniciado).
- Uma publicação ou chamada que faz o estado **crescer** paga o depósito que falta, até o `max_deposit` da transação (padrão da carteira: 1 TCN).
- Uma chamada que faz o estado **encolher** recebe `depósito × liberado ÷ tamanho antigo` de volta. Gravar um valor padrão apaga uma entrada, então limpar dados é recompensado.
- `destroy(to)` paga o depósito inteiro e o saldo a `to`.

O simulador mostra depósitos como estimativas e não os desconta dos saldos fictícios.

## Desempenho medido

A TCCL é um interpretador que percorre a árvore do programa, escrito em Rust. Rust traz segurança de memória e desempenho previsível; não executa contratos em velocidade nativa. O que importa para a rede é que o combustível acompanhe o tempo de CPU.

Medido com `cargo test -p tccl --release --test perf -- --ignored --nocapture` num Intel Xeon E5-1620 v2 @ 3,70 GHz (o armazenamento em memória do simulador é mais rápido que o disco de um nó, então as linhas de armazenamento são otimistas):

| Carga | Linguagem | ns por combustível | Pico de memória | Bloco de 50 M |
|---|---|---|---|---|
| Laço aritmético | 1 · 2 | 16,4 · 17,6 | < 0,1 MB | 0,8 s · 0,9 s |
| Escritas em map | 1 · 2 | 1,4 · 1,2 | 0,4 MB | 0,1 s |
| Cadeia de BLAKE3 | 1 · 2 | 5,0 · 4,7 | < 0,1 MB | 0,2 s |
| Construir records | 2 | 13,8 | < 0,1 MB | 0,7 s |
| Copiar uma lista de 4 000 textos (adversarial) | 1 · 2 | **2 109** · 12,2 | 0,3 MB | **105 s** · 0,6 s |
| Decodificar uma lista gravada (adversarial) | 1 · 2 | **1 656** · 21,1 | 0,4 MB | **83 s** · 1,1 s |
| Manter listas locais grandes (adversarial) | 1 · 2 | 10,1 · 6,4 | **65,6 MB** · 16,8 MB (limite) | 0,5 s · 0,3 s |

Seus números vão ser diferentes; rode `tccl bench` para uma verificação rápida na sua máquina.

# Introdução

A TCCL — **The Coin Cloud Language** — é a linguagem de contratos inteligentes da [The Coin](https://the-coin.cloud). Ela se lê como Python: os blocos são indentados, há poucas palavras-chave e um primeiro contrato cabe em vinte linhas. Por baixo, é rigorosa: todo valor tem tipo declarado, todo passo custa combustível, a aritmética é verificada e o mesmo código produz exatamente o mesmo resultado em todos os nós.

```tccl counter.tccl
contract Counter

state count: int

action increment(by: int):
    require by > 0, "by must be positive"
    count += by

view get() -> int:
    return count
```

## Princípios

- **Legível em primeiro lugar.** Um contrato é um único arquivo que qualquer pessoa pode revisar. Não há conversões implícitas, `null`, ponto flutuante nem fluxo de controle escondido.
- **Segura por construção.** Estouro, divisão por zero, falta de combustível e requisitos não atendidos interrompem a chamada e revertem todas as mudanças. Reentrada é impossível. Essas proteções fazem parte do motor e não podem ser desligadas.
- **Honesta sobre os limites.** Nada numa blockchain pública é secreto ou aleatório por si só, e um interpretador não é código nativo. A documentação diz isso e mostra os padrões que funcionam.
- **Compatível com o histórico.** O compilador faz parte das regras de consenso: uma publicação leva o código-fonte e cada nó o compila. Versões antigas da linguagem ficam congeladas para que blocos passados sempre possam ser reexecutados.

## Versões da linguagem e onde rodam {#versions}

| Versão | Situação | O que traz |
|---|---|---|
| **1** | Em uso na The Coin v0.2.0 (mainnet, testnet, regtest) | Tipos, estado, actions, views, eventos, combustível, pagamentos em TCN, depósitos de armazenamento, assinaturas em anel |
| **2** | Esta versão: compilador, CLI, simulador e playground. **Ainda não ativa na rede** | Records, enums com transições permitidas, papéis e `only`, interfaces e chamadas entre contratos, módulos e biblioteca padrão, autoridade de upgrade, `mul_div`/`isqrt`/`pow`, limites de memória, cópias com novo preço |

Todo contrato válido na versão 1 também é válido na versão 2 e se comporta da mesma forma (veja [Versões](versions.md)). Enquanto a rede não ativar a versão 2, publique na The Coin usando `tccl check --language 1`. O plano de ativação é público: [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

## Por onde começar

- **Novo em contratos inteligentes?** Siga [Seu primeiro contrato](tutorial.md) e depois abra o [playground](/pt-BR/playground/) para mexer nos exemplos.
- **Vem de outra linguagem?** Leia a [referência da linguagem](language.md) e o [modelo de segurança](security.md) — principalmente o que `caller` significa e por que toda falha reverte tudo.
- **Vai construir pagamentos, jogos ou uma corretora?** Veja [Chamar outros contratos](calls.md), [Permissões](permissions.md), a [biblioteca padrão](modules.md) e as [receitas testadas](recipes.md): um token, um pool de troca de produto constante, um jogo com commit–reveal, pagamentos condicionais e mais.
- **Opera a rede ou revisa o motor?** Leia a [arquitetura](architecture.md), [combustível e taxas](fees.md) e o [modelo de upgrade](upgrades.md).

## O que há nesta documentação

| Página | Conteúdo |
|---|---|
| [Seu primeiro contrato](tutorial.md) | Instalar, criar, verificar, testar, simular e publicar |
| [Referência da linguagem](language.md) | Sintaxe, tipos, instruções, records, enums, papéis, interfaces, funções embutidas, limites |
| [Módulos e biblioteca padrão](modules.md) | Módulos versus contratos publicados, `std.token`, `std.items`, `std.payments` |
| [Chamar outros contratos](calls.md) | Interfaces, identidade de quem chama, transferências, retornos, atomicidade, reentrada |
| [Permissões](permissions.md) | Papéis, `only`, `grant`, `revoke` e padrões comuns |
| [Upgrades](upgrades.md) | Autoridade de upgrade, regras de compatibilidade, `upgrade()`, tornar o código final |
| [Segurança](security.md) | Determinismo, falhas e taxas, limites, privacidade, dados externos, aleatoriedade, checklist |
| [Combustível, taxas e depósitos](fees.md) | Tabela de combustível, fórmula da taxa, depósitos, medições |
| [Referência de erros](errors.md) | Todos os erros de compilação e execução com explicação e correção |
| [Receitas testadas](recipes.md) | Contratos completos com cenários que rodam na integração contínua |
| [Ferramentas](tools.md) | CLI, cenários, playground, instaladores, publicação com a carteira |
| [Arquitetura](architecture.md) | Compilador, formato do programa, máquina virtual, interface do host, testes |
| [Versões](versions.md) | Versões da linguagem e da ferramenta, política de compatibilidade, mudanças |

A TCCL é software livre, com licença dupla MIT ou Apache-2.0. Código-fonte: [github.com/LucasBolla94/tccl](https://github.com/LucasBolla94/tccl).

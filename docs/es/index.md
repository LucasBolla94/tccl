# Introducción

TCCL — **The Coin Cloud Language** — es el lenguaje de contratos inteligentes de [The Coin](https://the-coin.cloud). Se lee como Python: los bloques van con sangría, hay pocas palabras clave y un primer contrato cabe en veinte líneas. Por debajo es estricto: cada valor tiene un tipo declarado, cada paso cuesta combustible, la aritmética se verifica y el mismo código produce exactamente el mismo resultado en todos los nodos.

```tccl counter.tccl
contract Counter

state count: int

action increment(by: int):
    require by > 0, "by must be positive"
    count += by

view get() -> int:
    return count
```

## Principios

- **Primero, legible.** Un contrato es un solo archivo que cualquiera puede revisar. No hay conversiones implícitas, `null`, coma flotante ni flujo de control oculto.
- **Seguro por construcción.** El desbordamiento, la división por cero, quedarse sin combustible y los requisitos incumplidos detienen la llamada y revierten todos los cambios. La reentrada es imposible. Estas protecciones forman parte del motor y no se pueden desactivar.
- **Honesto sobre sus límites.** Nada en una cadena pública es secreto o aleatorio por sí mismo, y un intérprete no es código nativo. La documentación lo dice y muestra los patrones que funcionan.
- **Compatible con la historia.** El compilador forma parte de las reglas de consenso: un despliegue lleva el código fuente y cada nodo lo compila. Las versiones antiguas del lenguaje quedan congeladas para que los bloques pasados siempre puedan volver a ejecutarse.

## Versiones del lenguaje y dónde se ejecutan {#versions}

| Versión | Estado | Qué aporta |
|---|---|---|
| **1** | En uso en The Coin v0.2.0 (mainnet, testnet, regtest) | Tipos, estado, actions, views, eventos, combustible, pagos en TCN, depósitos de almacenamiento, firmas en anillo |
| **2** | Esta versión: compilador, CLI, simulador y playground. **Aún no activa en la red** | Records, enums con transiciones permitidas, roles y `only`, interfaces y llamadas entre contratos, módulos y biblioteca estándar, autoridad de actualización, `mul_div`/`isqrt`/`pow`, límites de memoria, copias con nuevo precio |

Todo contrato válido en la versión 1 también es válido en la versión 2 y se comporta igual (vea [Versiones](versions.md)). Mientras la red no active la versión 2, despliegue en The Coin usando `tccl check --language 1`. El plan de activación es público: [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

## Por dónde empezar

- **¿Nuevo en contratos inteligentes?** Siga [Su primer contrato](tutorial.md) y luego abra el [playground](/es/playground/) para modificar los ejemplos.
- **¿Viene de otro lenguaje?** Lea la [referencia del lenguaje](language.md) y el [modelo de seguridad](security.md) — sobre todo qué significa `caller` y por qué todo fallo revierte todo.
- **¿Va a construir pagos, juegos o un exchange?** Vea [Llamar a otros contratos](calls.md), [Permisos](permissions.md), la [biblioteca estándar](modules.md) y las [recetas probadas](recipes.md): un token, un pool de intercambio de producto constante, un juego con commit–reveal, pagos condicionales y más.
- **¿Opera la red o revisa el motor?** Lea la [arquitectura](architecture.md), [combustible y comisiones](fees.md) y el [modelo de actualización](upgrades.md).

## Qué contiene esta documentación

| Página | Contenido |
|---|---|
| [Su primer contrato](tutorial.md) | Instalar, crear, verificar, probar, simular y desplegar |
| [Referencia del lenguaje](language.md) | Sintaxis, tipos, instrucciones, records, enums, roles, interfaces, funciones integradas, límites |
| [Módulos y biblioteca estándar](modules.md) | Módulos frente a contratos desplegados, `std.token`, `std.items`, `std.payments` |
| [Llamar a otros contratos](calls.md) | Interfaces, identidad de quien llama, transferencias, retornos, atomicidad, reentrada |
| [Permisos](permissions.md) | Roles, `only`, `grant`, `revoke` y patrones comunes |
| [Actualizaciones](upgrades.md) | Autoridad de actualización, reglas de compatibilidad, `upgrade()`, hacer el código final |
| [Seguridad](security.md) | Determinismo, fallos y comisiones, límites, privacidad, datos externos, aleatoriedad, lista de control |
| [Combustible, comisiones y depósitos](fees.md) | Tabla de combustible, fórmula de la comisión, depósitos, mediciones |
| [Referencia de errores](errors.md) | Todos los errores de compilación y ejecución con explicación y corrección |
| [Recetas probadas](recipes.md) | Contratos completos con escenarios que se ejecutan en integración continua |
| [Herramientas](tools.md) | CLI, escenarios, playground, instaladores, despliegue con la billetera |
| [Arquitectura](architecture.md) | Compilador, formato del programa, máquina virtual, interfaz del host, pruebas |
| [Versiones](versions.md) | Versiones del lenguaje y de la herramienta, política de compatibilidad, cambios |

TCCL es software libre con licencia doble MIT o Apache-2.0. Código fuente: [github.com/LucasBolla94/tccl](https://github.com/LucasBolla94/tccl).

# Recetas probadas

Cada receta es un contrato completo de la carpeta [`examples/`](https://github.com/LucasBolla94/tccl/tree/main/examples) con un escenario que comprueba su comportamiento, incluidos los fallos. Todos los escenarios se ejecutan en integración continua (`cargo test` y `tccl test examples`), así que el código de esta página funciona con esta versión.

Ejecute uno usted mismo:

```sh
git clone https://github.com/LucasBolla94/tccl && cd tccl/examples
tccl test counter.scenario
```

O abra el contrato en el [playground](/es/playground/): el escenario se carga en la pestaña *Escenario*.

## Contador

Estado, actions, views y eventos — el contrato útil más pequeño. Versión 1 del lenguaje, desplegable hoy en The Coin.

{{example:counter.tccl}}

{{scenario:counter.scenario}}

## Bote de propinas

Una action payable, un retiro solo para el dueño y una view que devuelve varios números. Versión 1.

{{example:tip_jar.tccl}}

{{scenario:tip_jar.scenario}}

## Token con rol de emisor

El módulo de token estándar más un rol. `transfer`, `approve`, `transfer_from` y las views vienen de `std.token`; el contrato solo decide quién puede emitir.

{{example:cloud_coin.tccl}}

{{scenario:cloud_coin.scenario}}

## Pedidos con records, transiciones y roles

Un record `Order` tipado, un enum `Status` cuyas transiciones permitidas impiden cambios imposibles (un pedido enviado no se puede cancelar) y dos roles.

{{example:orders.tccl}}

{{scenario:orders.scenario}}

## Pool de intercambio {#exchange-pool}

Un exchange de producto constante (DEX) para dos contratos de token con una comisión del 0,30 %. Muestra interfaces, `caller` dentro de otro contrato (los operadores aprueban al pool), `mul_div` e `isqrt`, protección contra deslizamiento con `min_out` y atomicidad: un swap sin autorización suficiente falla en el token y revierte la actualización de reservas del pool.

{{example:pool.tccl}}

{{scenario:pool.scenario}}

> Los exchanges reales también necesitan protección contra la manipulación del precio dentro de un bloque cuando otros contratos usan el precio del pool. No use `quote` como oráculo de precios.

## Juego de cara o cruz {#coin-flip-game}

Un juego para dos jugadores sin aleatoriedad falsa: el anfitrión se compromete con una elección oculta, el invitado adivina y el anfitrión revela. Un anfitrión que se niega a revelar pierde después de 20 bloques. Vea [Aleatoriedad](security.md#randomness).

{{example:coin_flip.tccl}}

{{scenario:coin_flip.scenario}}

## Entradas como objetos únicos

Entradas emitidas por un organizador con `std.items`, capacidad limitada, transferencias y control de acceso (quemando la entrada).

{{example:tickets.tccl}}

{{scenario:tickets.scenario}}

## Pagos condicionales

Todas las actions vienen de `std.payments`: un pago bloqueado por hash que se libera cuando alguien revela el secreto, y un pago que el comprador puede recuperar después de un plazo.

{{example:deals.tccl}}

{{scenario:deals.scenario}}

## Actualizar un contrato {#upgrading-a-contract}

El contador de arriba, actualizado a una segunda versión por su autoridad de actualización. La nueva variable de estado se inicializa en `upgrade()`, los valores antiguos se conservan, los extraños no pueden actualizar y un contrato final nunca vuelve a cambiar.

{{example:counter_v2.tccl}}

{{scenario:upgrade.scenario}}

## Depósito en garantía con árbitro

Roles expresados con `require`, plazos, una disputa y `destroy`. Versión 1.

{{example:escrow.tccl}}

{{scenario:escrow.scenario}}

## Más recetas {#more-recipes}

Estos contratos de la versión 1 se prueban en `crates/tccl/tests/examples.rs`:

| Contrato | Muestra |
|---|---|
| [`shop.tccl`](../../examples/shop.tccl) | Todo tipo de declaración, payable, auxiliares |
| [`token.tccl`](../../examples/token.tccl) | Un token escrito a mano: maps, autorizaciones, claves compuestas |
| [`crowdfund.tccl`](../../examples/crowdfund.tccl) | Plazos y reembolsos |
| [`poll.tccl`](../../examples/poll.tccl) | Argumentos lista, listas de estado, bucles acotados |
| [`savings.tccl`](../../examples/savings.tccl) | Bloqueos por tiempo y reembolsos de almacenamiento |
| [`treasury.tccl`](../../examples/treasury.tccl) | Aprobaciones M de N |
| [`names.tccl`](../../examples/names.tccl) | Claves de texto, validación, vencimiento |
| [`private_pool.tccl`](../../examples/private_pool.tccl) | Firmas en anillo para pagos privados (vea [Privacidad](security.md#privacy)) |

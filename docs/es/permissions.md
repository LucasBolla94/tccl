# Permisos

Quién puede hacer qué es la pregunta más importante de cualquier contrato. La versión 2 del lenguaje hace la respuesta explícita en el código y visible en la interfaz.

## Roles

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

- `role nombre` declara un conjunto de direcciones. Un contrato puede declarar varios roles; un módulo puede declarar los suyos.
- `grant nombre to <dirección>` agrega una dirección; `revoke nombre from <dirección>` la quita. Ambas son instrucciones y emiten `RoleGranted(role, account, by)` o `RoleRevoked(role, account, by)`, así que el historial de permisos queda en los recibos.
- `nombre.has(dirección)` indica si la dirección tiene el rol.
- Un rol no es un valor: no se puede asignar, pasar ni devolver.
- Nadie tiene un rol hasta que el contrato lo concede. Conceda el primer administrador en `init()`.

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

- `only a, b, …` va al final del encabezado de la función. La llamada continúa si `caller` tiene alguno de los roles enumerados o es igual a alguna **variable de estado de tipo address** enumerada. Si no, falla con `requirement failed: only owner or treasurer can call 'withdraw'` y nada cambia.
- Funciona en actions, auxiliares `fn` y `upgrade()`. No se permite en views (las consultas gratuitas no tienen un llamante verificado) ni en `init()` (todavía no existe ningún rol).
- La verificación se ejecuta antes de la primera instrucción de la función, así que no se hace trabajo para quien no tiene permiso.
- `tccl check`, `tccl abi` y el playground muestran las reglas junto a cada función, por ejemplo `action withdraw(to: address, amount: int) only owner, treasurer`.

`only owner` es exactamente `require caller == owner, "only owner can call '…'"` escrito de forma que las herramientas lo puedan leer. Los contratos de la versión 1 usan la forma con `require`.

## Patrones

**Propiedad transferible**

```tccl
state owner: address
state pending_owner: address

action offer_ownership(to: address) only owner:
    pending_owner = to

action accept_ownership() only pending_owner:
    owner = pending_owner
    pending_owner = zero_address()
```

Los dos pasos evitan entregar el contrato a una dirección mal escrita.

**Nunca se deje fuera.** Revocar al último administrador es permanente. Lleve una cuenta si importa:

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

**Roles para contratos.** Un rol se puede conceder a la dirección de un contrato. En una llamada desde ese contrato, `caller` es el contrato, así que `only operator` también funciona para contratos — por ejemplo un juego autorizado a crear objetos.

**Varias aprobaciones.** Para decisiones que necesitan M de N personas, guarde las aprobaciones por propuesta y actúe cuando la cuenta llegue a M — vea [`treasury.tccl`](recipes.md#more-recipes).

## Errores a evitar

- **Autorizar con `origin`.** Use siempre `caller` (u `only`). `origin` es quien firmó la transacción aunque haya un contrato desconocido en medio.
- **Inicialización sin protección.** Fije dueños y conceda roles en `init()`, que se ejecuta una vez al desplegar. Una action `setup` que cualquiera puede llamar primero es una toma de control.
- **Olvidar que las views son públicas.** `only` no oculta datos: todo lo guardado en la cadena lo puede leer cualquiera.
- **Confiar a ciegas en la autoridad de actualización.** La autoridad de un contrato actualizable puede reemplazar su código, incluidas las verificaciones de permisos. Vea [Actualizaciones](upgrades.md).

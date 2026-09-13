# Actualizaciones

A veces el código desplegado necesita correcciones. TCCL usa una **autoridad de actualización**: cada contrato tiene como máximo una dirección autorizada a reemplazar su código, y esa dirección puede ceder su poder o renunciar a él para siempre. Nada se actualiza de forma implícita — cada actualización es una transacción firmada por la autoridad.

> **Estado en la red.** Las actualizaciones las define la versión 2 del lenguaje y funcionan en `tccl`, los escenarios y el playground. The Coin v0.2.0 no tiene transacción de actualización: los contratos desplegados allí hoy no pueden cambiar. Activarlas requiere un cambio de protocolo descrito en [implant-the-coin-language.md](https://github.com/LucasBolla94/tccl/blob/main/implant-the-coin-language.md).

## El modelo

| | |
|---|---|
| Al desplegar | Quien despliega pasa a ser la autoridad de actualización, salvo que el contrato se despliegue como **final** (`--final`) |
| Actualizar | La autoridad envía el nuevo código fuente; reemplaza el código **de inmediato** si es compatible |
| Cambiar la autoridad | La autoridad puede cederla a otra dirección, por ejemplo una multifirma o un contrato de gobernanza |
| Renunciar | La autoridad puede fijarla en ninguna: el contrato pasa a ser **final** para siempre |
| Contratos finales | Nunca más se pueden actualizar |

Es flexible — un equipo puede corregir un error el mismo día — y esa flexibilidad también es el riesgo: **quien tiene la autoridad controla el contrato**, incluidos sus fondos y permisos. Los usuarios deben tratar un contrato actualizable como si confiaran en su autoridad.

## Qué pueden verificar los usuarios y otros contratos

- `tccl run … state` y el playground muestran la autoridad de actualización y la versión del código.
- Otros contratos pueden exigir dependencias inmutables: `require is_final(token), "token must be final"`.
- `code_hash(addr)` identifica el código exacto; compárelo con el hash del código que revisó.

Buenas prácticas para proyectos que conservan una autoridad:

- guardarla en una multifirma o un contrato de gobernanza, no en una sola clave en línea;
- anunciar las actualizaciones y publicar el nuevo código antes de enviarlas;
- hacer el contrato final cuando esté estable.

## Reglas de compatibilidad

El almacenamiento sobrevive a la actualización, así que el nuevo código debe leer los datos antiguos de la misma forma. `tccl` comprueba, antes de ejecutar nada:

- cada variable de estado del código antiguo sigue existiendo **con el mismo nombre y un tipo compatible** — no se puede eliminar ni cambiar el tipo de una variable (deje de usarla);
- las variables de estado nuevas se agregan después de las existentes (el compilador mantiene automáticamente cada variable existente en su posición de almacenamiento, en el orden en que las escriba);
- los records guardados en el estado conservan **exactamente los mismos campos** en el mismo orden;
- los enums guardados en el estado conservan sus variantes en orden; las nuevas solo pueden **agregarse al final**;
- la versión del lenguaje no retrocede.

Todo lo demás — funciones, eventos, constantes, reglas de roles, records nuevos — puede cambiar. El informe de actualización enumera el estado agregado, las funciones agregadas, eliminadas y cambiadas, y notas como *"the new code can send TCN (the old code could not)"*.

## `upgrade()` — migrar datos

```tccl
contract Counter

state count: int
state last_caller: address
state step: int                      # nuevo en esta versión

upgrade():
    step = 10                        # se ejecuta una vez, dentro de la transacción de actualización

action increment(amount: int):
    count += amount * step
```

- `upgrade(params)` se ejecuta una vez cuando la actualización instala el código, llamado por la autoridad (`caller`). Puede recibir parámetros y usar `only`.
- Si falla, se revierte toda la actualización y el código antiguo se mantiene.
- No se puede llamar después: no es una action.

## En la práctica

```sh
tccl run counter.tccl deploy --from dev               # dev es la autoridad de actualización
tccl run counter_v2.tccl upgrade --from dev
tccl run counter.tccl authority @multisig --from dev  # cede la autoridad
tccl run counter.tccl authority none --from multisig  # hace el contrato final
```

En un escenario:

```scenario upgrade.scenario
deploy counter.tccl as counter --from dev
upgrade counter counter_v2.tccl --from bob
expect fail
upgrade counter counter_v2.tccl --from dev
expect ok
authority counter none --from dev
```

La receta completa y probada es [Actualizar un contrato](recipes.md#upgrading-a-contract).

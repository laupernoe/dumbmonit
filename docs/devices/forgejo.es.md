# Forgejo / Gitea

Forgejo y Gitea comparten la misma API REST, así que DumbMonit lee ambos
con un único recolector: si la instancia responde, su comprobación de
salud, cuántos repositorios, cuentas y organizaciones contiene, si sus
tareas programadas (comprobaciones de repositorio, limpiezas, el
comprobador de actualizaciones) siguen ejecutándose en su propio
intervalo, y si sus runners de Actions están en línea.

Dos llamadas deben tener éxito: `/api/v1/version`, pública, y
`/api/v1/user`, la prueba de que el **token de acceso** es válido. Todo lo
demás — la comprobación de salud, los recuentos de repositorios, cuentas y
organizaciones, las tareas programadas y los runners de Actions — se lee
de endpoints que Forgejo y Gitea reservan a cuentas de administrador del
sitio, y se omite sin hacer fallar la sonda cuando el token pertenece a una
cuenta corriente.

El fallo que vale la pena vigilar aquí: **una tarea programada que dejó de
ejecutarse**. Forgejo y Gitea siguen sirviendo páginas y operaciones git
mientras su planificador interno está atascado; nadie se da cuenta hasta
que el comprobador de actualizaciones o una comprobación de integridad de
repositorio no se ha ejecutado en semanas.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_forgejo_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `token_is_admin` | 1 cuando la cuenta del token es administradora del sitio | |
| `healthz_ok` | 1 a menos que la comprobación de salud reporte `fail` | |
| `healthz_check` | 1 por componente fallido (base de datos, caché) | `check` |
| `repos` | repositorios visibles para la cuenta del token | |
| `users`, `orgs` | cuentas y organizaciones en la instancia | |
| `cron_tasks` | tareas programadas definidas en la instancia | |
| `cron_task_overdue` | 1 por tarea cuya próxima ejecución ya ha vencido, 0 en caso contrario; las tareas desactivadas (programación `-`) se omiten | `task` |
| `runners_total`, `runners_online`, `runners_offline` | runners de Actions conocidos por la instancia; `online`, `idle` y `active` cuentan como en línea | |
| `runner_offline` | valor 1 por runner fuera de línea (como máximo 32) | `runner` |
| `scrape_errors` | llamadas solo de administrador que fallaron en la última sonda (0 con las de un token corriente, nunca intentadas) | |

Las [reglas integradas](../alerting/rules.md#self-hosted-forges) que se
aplican: Forgejo/Gitea runners all offline, Forgejo/Gitea scheduled task
overdue, además de Device unreachable. Una puntuación de seguridad
comprueba las tareas programadas: consulta [Actualizaciones de proveedor y
otras comprobaciones](../using/security-score.md).

## Qué no vigila {#what-it-does-not-watch}

Forgejo y Gitea no tienen ningún rol de administrador de solo lectura:
cada llamada reservada a «admin» necesita una cuenta con todos los
derechos, no un delegado con alcance limitado. Su API tampoco tiene un
equivalente de los ajustes de registro o doble factor de GitLab, ni un
indicador de «actualización disponible» autoinformado — compara la
versión de arriba con las [notas de versión](https://forgejo.org/releases/)
del proyecto a mano.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca llama a Forgejo ni a Gitea. Indica primero qué va mal (la
comprobación de salud fallando, runners todos fuera de línea, una tarea
programada vencida), luego la versión, y después los recuentos de
repositorios, cuentas y organizaciones.

## Crear un token de acceso en una cuenta de administrador del sitio {#create-an-access-token-on-a-site-administrator-account}

1. Crea un token de acceso en una cuenta de administrador del sitio:
   avatar → Settings → Applications → Generate new token. Nómbralo como
   sigue y marca los alcances read:admin, read:repository y read:user.

    ```
    dumbmonit
    ```

2. read:admin abre los recuentos de administración, las tareas
   programadas y los runners de Actions; sin él, solo se leen la versión y
   la propia cuenta del token, y el resto se omite sin hacer fallar la
   sonda. Forgejo y Gitea tampoco tienen ningún rol de administrador de
   solo lectura.

3. En DumbMonit, introduce la dirección, por ejemplo «forgejo.lan» (puerto
   3000) o «https://git.example.com» detrás de un proxy inverso, y pega el
   token.

!!! warning
    Un token de acceso en una cuenta de administrador del sitio puede leer
    y cambiar todo lo que la cuenta puede, cualesquiera que sean los
    alcances marcados: Forgejo y Gitea solo controlan qué rutas de la API
    abre un alcance, no cuánto puede hacer la propia cuenta
    administradora. Dedícale una cuenta bot en vez de reutilizar la tuya.

Comprobar el token a mano, desde el host de DumbMonit:

```
curl --header 'Authorization: token YOUR-TOKEN' 'https://git.example.com/api/v1/version'
```

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Token de acceso | Enviado como `Authorization: token …`. |

Dirección: un nombre de host (`forgejo.lan`) o `host:puerto`. Opciones:
protocolo (HTTP por defecto — pon un proxy inverso con TLS delante para
cualquier cosa que no sea una LAN), puerto (3000), comprobación de
certificado y tiempo de espera de la petición (15 s).

Validado contra Forgejo 9.0 (código fuente oficial de la API y
definiciones Swagger, compartidas con Gitea).

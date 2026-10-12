# GitLab (autoalojado) {#gitlab-self-managed}

Si una instancia de GitLab autoalojada está sirviendo, y si las partes que
silenciosamente sostienen todo lo demás van al día: las colas de Sidekiq,
los runners de CI/CD, las migraciones de base de datos pendientes de una
actualización y — para GitLab Enterprise Edition — la licencia.

DumbMonit lee `/api/v4/version` con un **token de acceso personal**, la
única llamada que debe tener éxito: es la prueba de que GitLab responde y
de que el token es válido. Todo lo demás — Sidekiq, los runners, las
migraciones, las estadísticas de la instancia, los ajustes de registro y
doble factor, la licencia, y la comprobación de disponibilidad en detalle
— se lee desde endpoints que GitLab reserva a cuentas de administrador, y
se omite sin hacer fallar la sonda cuando el token pertenece a una cuenta
corriente.

El fallo que vale la pena vigilar aquí: **Sidekiq quedándose atrás**.
GitLab sigue sirviendo páginas mientras la cola de workers en segundo
plano crece, y nada en la página lo indica hasta que una notificación, un
webhook o una actualización de pipeline de CI se retrasa minutos u horas.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_gitlab_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version`, `revision` |
| `readiness_check` | 1 cuando un componente de disponibilidad (base de datos, caché, colas, estado compartido, Gitaly) no está en `ok` | `check` |
| `sidekiq_queue_backlog`, `sidekiq_queue_latency_seconds` | trabajos esperando y su antigüedad, por cola de Sidekiq | `queue` |
| `runners_total`, `runners_online`, `runners_offline` | runners de CI/CD conocidos por la instancia | |
| `runner_offline` | valor 1 por runner fuera de línea (como máximo 32) | `runner` |
| `migrations_pending` | migraciones de base de datos pendientes de aplicar | |
| `migration_pending` | valor 1 por migración pendiente (como máximo 32) | `migration` |
| `projects`, `users`, `groups`, `forks`, `issues`, `merge_requests` | recuentos de toda la instancia | |
| `signup_enabled` | 1 mientras el registro público está abierto | |
| `two_factor_required` | 1 mientras todo usuario debe configurar la autenticación de doble factor | |
| `license_info` | valor 1 (solo Enterprise Edition) | `plan` |
| `license_expired`, `license_expiry_seconds` | si la licencia ha caducado, y el tiempo restante | |
| `pipelines_failed_recent`, `pipelines_pending` | pipelines fallidos en la ventana de observación, y pipelines pendientes o en ejecución ahora — solo para los proyectos listados en la opción «Projects watched for CI health» | `project` |
| `scrape_errors` | llamadas solo de administrador que fallaron en la última sonda (0 con las de un token corriente, nunca intentadas) | |

Las [reglas integradas](../alerting/rules.md#self-hosted-forges) que se
aplican: GitLab runners all offline, GitLab Sidekiq backlog high, GitLab
migrations pending, GitLab license expiring, además de Device
unreachable. Una puntuación de seguridad comprueba los ajustes de doble
factor y registro, y las migraciones pendientes: consulta
[Actualizaciones de proveedor y otras comprobaciones](../using/security-score.md).

## Qué no vigila {#what-it-does-not-watch}

La propia API REST de GitLab no tiene ningún endpoint que diga «hay una
versión de GitLab más reciente disponible»: a diferencia de Nextcloud o
Paperless-ngx, no hay ningún indicador de actualización autoinformado que
leer, así que DumbMonit no afirma tener uno. Compara la versión de arriba
con la [página de versiones](https://about.gitlab.com/releases/) de
GitLab a mano, o vigila el propio aviso de comprobación de actualizaciones
de GitLab en el área de administración.

El estado de los pipelines de CI solo se lee para los proyectos que
listes: la API de GitLab no tiene ningún endpoint de «todos los pipelines
fallidos» a nivel de instancia, solo uno por proyecto.

## La página del dispositivo {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca llama a GitLab. Indica primero qué va mal (un componente de
disponibilidad fallido, runners todos fuera de línea, un backlog grande
de Sidekiq), luego la versión, y después los recuentos de la instancia y
los ajustes leídos desde la API de administración.

## Crear un token de acceso personal en una cuenta de administrador {#create-a-personal-access-token-on-an-administrator-account}

1. Crea un token de acceso personal en una cuenta de administrador:
   avatar → Edit profile → Access tokens → Add new token. Nómbralo como
   sigue, define una fecha de caducidad que te resulte cómodo renovar, y
   marca solo el alcance read_api: lee toda la API y no escribe nada.

    ```
    dumbmonit
    ```

2. Las colas de Sidekiq, los runners de CI, las migraciones pendientes,
   las estadísticas de la instancia, los ajustes de registro y doble
   factor y la licencia se leen cada uno desde un endpoint solo para
   administradores: GitLab no tiene una delegación más fina para ellos.
   Un token de una cuenta corriente sigue funcionando; entonces solo se
   lee la versión, y el resto se omite sin hacer fallar la sonda.

3. En DumbMonit, introduce la dirección con la que abres GitLab, por
   ejemplo «gitlab.example.com» (HTTPS en 443) o «https://git.example.com»
   detrás de un proxy inverso, y pega el token.

4. Para vigilar también los pipelines de CI, lista los IDs numéricos de
   proyecto (mostrados en la página de resumen de cada proyecto, bajo su
   nombre) en la opción «Projects watched for CI health», separados por
   comas.

!!! warning
    Un token de acceso personal en una cuenta de administrador puede leer
    todo lo que la cuenta puede leer, incluso con el alcance read_api:
    GitLab no tiene ningún rol de administrador de solo lectura. Dedícale
    una cuenta bot en vez de reutilizar la tuya, y revoca el token si
    alguna vez se filtra.

Comprobar el token a mano, desde el host de DumbMonit:

```
curl --header 'PRIVATE-TOKEN: YOUR-TOKEN' 'https://gitlab.example.com/api/v4/version'
```

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Token de acceso personal | Enviado como la cabecera `PRIVATE-TOKEN`. |

Dirección: un nombre de host (`gitlab.example.com`) o `host:puerto`.
Opciones: protocolo (HTTPS por defecto), puerto (443), comprobación de
certificado, tiempo de espera de la petición (15 s), los proyectos
vigilados para la salud de CI (ninguno por defecto) y la ventana de
observación de pipelines fallidos (24 h).

Validado contra GitLab Enterprise Edition 17.5 (documentación oficial de
la API).

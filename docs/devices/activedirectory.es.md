# Active Directory

Un dominio Windows a través de LDAP, de solo lectura: los controladores
de dominio y quién tiene cada rol FSMO, los miembros de los grupos
privilegiados, la política de contraseñas, recuentos de cuentas, y los
hechos de seguridad en bruto que una auditoría busca primero — cuentas
expuestas a Kerberoasting o AS-REP roasting, delegación sin restricción,
una contraseña krbtgt antigua, equipos sin LAPS, versiones de Windows
fuera de soporte.

DumbMonit se vincula con una cuenta de servicio dedicada que es un
**usuario de dominio corriente, sin ningún grupo**. En Active Directory
cada usuario autenticado puede leer lo que lee DumbMonit; no se necesita
ningún derecho administrativo, y no debería darse ninguno. DumbMonit
nunca escribe en el directorio, nunca lee un atributo de contraseña (de
LAPS lee la fecha de caducidad, nunca la contraseña) y nunca pide los
derechos de replicación que expondrían secretos.

Un dispositivo cubre todo el dominio: apúntalo a un controlador de
dominio y DumbMonit encuentra los demás en la partición de
configuración.

## Qué puede y no puede contar LDAP {#what-ldap-can-and-cannot-tell}

LDAP muestra el directorio tal como es, no los registros de lo que pasó
en él. En particular:

- **Replicación.** El controlador de dominio consultado publica el
  estado de sus socios de replicación entrantes (último éxito, fallos
  consecutivos, último error) solo a una cuenta con el derecho *Monitor
  active directory replication*. Sin él, el panel dice *replication
  status not readable* en lugar de adivinar. Solo se muestra la vista
  del controlador de dominio consultado: para ver los socios de otro,
  añádelo como un segundo dispositivo.
- **No visible por LDAP**: registros de eventos (inicios de sesión,
  bloqueos en el momento, cambios de grupo como eventos), contenido de
  Group Policy, SYSVOL, zonas DNS, y los propios hashes de contraseña.
- **Las fechas de último inicio de sesión** vienen de
  `lastLogonTimestamp`, que los controladores de dominio replican solo
  cada 9 a 14 días: una cuenta contada como inactiva es inactiva con un
  margen de dos semanas.
- **La membresía de grupo** se lee con anidamiento (la regla
  `LDAP_MATCHING_RULE_IN_CHAIN`), a través de `member`/`memberOf`. Un
  grupo primario definido por `primaryGroupID` no es un enlace `member`
  y no se ve.

## Cómo lee {#how-it-reads}

Dos ritmos:

- **En cada comprobación** (cada minuto por defecto): conexión y bind,
  el RootDSE (niveles funcionales, reloj, sincronización), el objeto de
  dominio (política de contraseñas, cuota de cuenta de máquina), los
  cinco roles FSMO, los controladores de dominio de la partición de
  configuración, los grupos privilegiados y sus miembros efectivos, la
  función Papelera de reciclaje, y una conexión TCP al puerto LDAP de
  cada otro controlador de dominio.
- **En segundo plano** cada `inventory_minutes` (15 por defecto): cada
  cuenta de usuario y equipo, solo con los atributos que necesitan los
  hallazgos, en páginas de 500. Enumerar todo el directorio cada minuto
  cargaría a los controladores de dominio para nada.

La primera comprobación espera unos segundos al inventario; en un
dominio grande, los hallazgos de cuenta aparecen en la siguiente
comprobación.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_ad_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `bind_ok` | 1 cuando la cuenta de servicio es aceptada, 0 cuando el controlador de dominio la rechaza | |
| `connect_seconds`, `bind_seconds` | tiempo para abrir la conexión (TLS incluido) y para hacer el bind | |
| `tls_verified` | 1 cuando se comprueba el certificado | |
| `domain_info` | valor 1 | `domain`, `forest`, `dc` |
| `domain_functional_level`, `forest_functional_level` | 0 (2000) a 7 (2016), 10 (2025) | |
| `dc_synchronized`, `dc_global_catalog_ready` | el controlador de dominio consultado ha terminado su replicación inicial, su catálogo global está listo | |
| `clock_skew_seconds` | reloj del controlador de dominio menos reloj del host DumbMonit | |
| `recycle_bin_enabled` | 1 cuando la Papelera de reciclaje de AD está activada | |
| `dc_count` | controladores de dominio del dominio | |
| `dc_info` | valor 1 | `dc`, `site`, `gc`, `rodc`, `os` |
| `dc_reachable` | 1 o 0 por controlador de dominio; sin serie cuando su nombre no resuelve desde DumbMonit | `dc` |
| `fsmo_role` | valor 1 | `role` (`schema`, `domain_naming`, `pdc`, `rid`, `infrastructure`), `holder` |
| `replication_readable` | 1 cuando se puede leer el estado de replicación entrante | |
| `replication_consecutive_failures`, `replication_last_success_age_seconds` | por socio entrante y partición | `source`, `naming_context` |
| `privileged_group_members`, `privileged_group_enabled_members` | miembros efectivos, anidamiento incluido, grupos excluidos | `group` |
| `privileged_group_fingerprint` | un hash de los miembros: cambia cuando alguien se une o se va | `group` |
| `users_total`, `users_enabled`, `users_disabled`, `users_locked_out`, `users_password_never_expires`, `users_stale` | cuentas de usuario | |
| `computers_total`, `computers_enabled`, `computers_stale`, `groups_total` | equipos y grupos | |
| `admincount_accounts` | usuarios protegidos por AdminSDHolder (`adminCount=1`) | |
| `krbtgt_password_age_seconds` | antigüedad de la contraseña krbtgt | |
| `laps_schema`, `laps_computers`, `laps_eligible_computers` | LAPS en el esquema; equipos no-DC activados con caducidad LAPS, de entre todos | |
| `password_min_length`, `password_history_length`, `password_max_age_seconds` (0 = nunca), `password_complexity`, `lockout_threshold`, `machine_account_quota` | política de dominio por defecto | |
| `finding_count` | objetos afectados por un hallazgo, 0 cuando se examinó y está limpio | `finding`, `severity`, `title` |
| `inventory_age_seconds`, `scrape_errors` | antigüedad del último inventario completo; partes del directorio que no se pudieron leer | |

Los grupos privilegiados se encuentran por su identificador bien
conocido, no por su nombre, así que un dominio instalado en otro idioma
funciona igual: Domain Admins (RID 512), Enterprise Admins (519) y
Schema Admins (518) — los dos últimos solo en el dominio raíz del
bosque — y los integrados Administrators, Account Operators, Server
Operators, Print Operators y Backup Operators.

## Hallazgos {#findings}

Los hallazgos son hechos en bruto — cuántos objetos, y cuáles — no una
puntuación; la [puntuación de seguridad](../using/security-score.md)
del dispositivo los pondera. Cada uno tiene un `id` estable, una
severidad en la escala de la puntuación (low, medium, high, critical,
mostradas en la página como Info, Advisory y Warning), una categoría,
un recuento y hasta veinte nombres de cuenta de ejemplo. Un hallazgo
examinado y sin nada encontrado se conserva con un recuento de 0, así
que un problema arreglado se pone en verde en lugar de desaparecer; los
hallazgos sobre cuentas aparecen una vez completado el primer inventario
en segundo plano. Se sirven tal cual por
`GET /api/targets/{id}/ad/findings` y se publican como
`dumbmonit_ad_finding_count{finding, severity, title}`.

| `id` | Severidad | Qué |
|---|---|---|
| `unconstrained_delegation` | critical | cuentas activadas de confianza para delegación sin restricción, controladores de dominio aparte |
| `kerberoastable_privileged` | critical | miembros activados de grupos privilegiados con un nombre principal de servicio |
| `asrep_roastable_users` | high | cuentas activadas con la preautenticación Kerberos desactivada (`DONT_REQ_PREAUTH`) |
| `kerberoastable_users` | high | usuarios activados con un nombre principal de servicio |
| `krbtgt_password_age` | high | contraseña krbtgt con más de 180 días |
| `laps_coverage` | high cuando LAPS está ausente, medium cuando es parcial | equipos activados (controladores de dominio aparte) sin contraseña LAPS |
| `reversible_encryption` | high | usuarios cuya contraseña se guarda con cifrado reversible |
| `guest_enabled` | high | la cuenta integrada Guest (RID 501) está activada |
| `obsolete_os` | high | equipos activados con Windows XP, Vista, 7, 8, 2000, Server 2003, 2008 o 2012 |
| `dc_unreachable` | high | controladores de dominio cuyo puerto LDAP no responde |
| `replication_failing` | high | socios de replicación entrante con fallos consecutivos |
| `dc_not_synchronized` | high | el controlador de dominio consultado no ha terminado su replicación inicial |
| `fsmo_role_orphaned` | high | un rol FSMO en manos de un controlador de dominio eliminado |
| `password_policy` | high por debajo de 8 caracteres o sin bloqueo, medium en caso contrario | brechas en la política de dominio por defecto frente a la referencia: 14 caracteres, un umbral de bloqueo, complejidad |
| `privileged_group_size` | medium | más de 5 cuentas activadas en Domain Admins, Enterprise Admins y Administrators juntos |
| `stale_accounts` | medium | usuarios y equipos activados inactivos por más de los umbrales |
| `protocol_transition_delegation` | medium | delegación restringida con transición de protocolo |
| `des_only` | medium | cuentas restringidas a claves Kerberos DES |
| `password_not_required` | medium | usuarios a los que se permite una contraseña vacía (`PASSWD_NOTREQD`) |
| `privileged_password_never_expires` | medium | usuarios privilegiados cuya contraseña nunca caduca |
| `privileged_stale` | medium | usuarios privilegiados inactivos por más de `stale_days_users` |
| `schema_admins_not_empty` | medium | Schema Admins tiene miembros |
| `functional_level_old` | medium | nivel funcional de dominio por debajo de 2012 R2 |
| `clock_skew` | medium | más de cinco minutos entre el controlador de dominio y DumbMonit |
| `windows10_end_of_support` | medium | equipos activados con Windows 10 |
| `admincount_orphans` | low | usuarios con `adminCount=1` que ya no están en un grupo privilegiado |
| `password_never_expires` | low | usuarios activados cuya contraseña nunca caduca |
| `machine_account_quota` | low | cualquier usuario puede unir equipos al dominio |
| `recycle_bin_disabled` | low | la Papelera de reciclaje de AD no está activada |

Las [reglas integradas](../alerting/rules.md#active-directory) que se
aplican:

- **Domain controller unreachable**: otro controlador de dominio del
  dominio deja de responder en su puerto LDAP durante cinco minutos
  (Warning). El consultado está cubierto por *Device unreachable*.
- **Active Directory bind failed**: el controlador de dominio rechaza
  la cuenta de servicio — contraseña caducada, desactivada o bloqueada
  (Advisory).
- **Privileged group membership changed**: alguien se unió o salió de
  un grupo privilegiado, anidamiento incluido (Advisory, se resuelve una
  hora después).
- **Active Directory replication failing**: un socio entrante falla
  durante media hora (Advisory), cuando el estado de replicación es
  legible.
- **krbtgt password older than 180 days** (Info).

## La página del dispositivo {#the-device-page}

El panel Active Directory muestra el dominio (niveles funcionales, el
controlador de dominio consultado, desfase de reloj), los controladores
de dominio con su sitio, roles y si responden, los grupos privilegiados
con sus miembros, la política de contraseñas y recuentos de cuentas, y
la tabla de hallazgos con palabras de severidad.

## Crear una cuenta de servicio de solo lectura para DumbMonit {#create-a-read-only-service-account-for-dumbmonit}

1. Crea un usuario dedicado para DumbMonit, por ejemplo svc-dumbmonit,
   y no lo añadas a ningún grupo. Un usuario de dominio corriente
   basta: cada usuario autenticado puede leer lo que lee DumbMonit. En
   PowerShell, con el módulo de Active Directory:

    ```
    New-ADUser -Name svc-dumbmonit -UserPrincipalName svc-dumbmonit@corp.example.com -AccountPassword (Read-Host -AsSecureString 'Password') -Enabled $true
    ```

2. Dale una contraseña larga y aleatoria. Si las contraseñas caducan en
   tu dominio, DumbMonit lanza Active Directory bind failed el día que
   caduque esta.

3. Opcional: para ver el estado de replicación entrante, concede a la
   cuenta el derecho Monitor active directory replication en el propio
   objeto de dominio. No revela ninguna contraseña ni ningún secreto.

    ```
    dsacls "DC=corp,DC=example,DC=com" /G "CORP\svc-dumbmonit:CA;Monitor active directory replication"
    ```

4. LDAPS necesita un certificado en cada controlador de dominio; una
   autoridad de certificación empresarial (AD CS) emite uno
   automáticamente. Exporta el certificado de esa autoridad en formato
   X.509 codificado en Base-64 y pégalo como CA certificate abajo.

5. En DumbMonit, introduce el nombre de un controlador de dominio tal
   como lo nombra su certificado, por ejemplo «dc1.corp.example.com», e
   inicia sesión como svc-dumbmonit@corp.example.com. DumbMonit
   encuentra los demás controladores de dominio por sí mismo y comprueba
   que cada uno responde.

6. DumbMonit solo lee: nunca escribe en el directorio, y de LAPS lee la
   fecha de caducidad, nunca la contraseña.

!!! warning
    El LDAP plano en el puerto 389 envía la contraseña de la cuenta de
    servicio en texto plano: DumbMonit lo rechaza a menos que marques
    Allow plain LDAP, y un controlador de dominio que requiere firma
    LDAP también lo rechaza. Usa LDAPS, o StartTLS en el puerto 389.

Para evitar que la cuenta abra una sesión en ningún sitio, deniégale
*Log on locally* y *Log on through Remote Desktop Services* por Group
Policy: un bind LDAP no se ve afectado. No la restrinjas con *Log On
To* ni *Deny access to this computer from the network*: los
controladores de dominio rechazarían entonces el bind.

### Mínimo privilegio, en detalle {#least-privilege-in-detail}

- La cuenta no necesita ningún grupo más allá de Domain Users, que
  tiene toda cuenta.
- No uses una cuenta de servicio administrada (gMSA): no puede hacer un
  bind LDAP simple.
- No concedas *Replicating Directory Changes* (ni *All*): ese es el
  derecho DCSync, que expone cada hash de contraseña. DumbMonit nunca lo
  necesita.
- *Monitor active directory replication* solo permite leer metadatos de
  replicación (socios, último éxito, códigos de error). Es opcional.

## LDAPS y certificados {#ldaps-and-certificates}

- **LDAPS (puerto 636)** es lo por defecto. Cada controlador de dominio
  de un dominio con una autoridad de certificación empresarial recibe
  un certificado automáticamente; sin una, el controlador de dominio no
  escucha en 636. El certificado debe llevar el nombre que usa
  DumbMonit: introduce el controlador de dominio por su nombre DNS, no
  su dirección IP, a menos que el certificado liste la dirección.
- **CA certificate**: pega el PEM de la autoridad raíz (o emisora).
  Solo se confía entonces en esa autoridad. Sin él, DumbMonit confía en
  las raíces públicas y el almacén del sistema de su contenedor, que no
  conocen una autoridad empresarial privada.
- **StartTLS (puerto 389)** cifra de la misma forma, tras un saludo en
  texto plano. Úsalo cuando el puerto 636 está filtrado.
- **Accept an unverifiable certificate** mantiene la conexión cifrada
  pero acepta cualquier certificado: cualquiera en el camino podría
  hacerse pasar por el controlador de dominio y recibir la contraseña de
  la cuenta de servicio.
- **LDAP channel binding** (`LdapEnforceChannelBinding`) concierne a
  los binds NTLM y Kerberos por TLS; el bind simple que usa DumbMonit no
  se ve afectado.

## Credenciales {#credentials}

| Campo | Valor |
|---|---|
| Nombre de usuario | `svc-dumbmonit@corp.example.com`, `CORP\svc-dumbmonit`, o el DN completo de la cuenta |
| Contraseña | la contraseña de la cuenta, guardada cifrada y nunca vuelta a mostrar |

Una contraseña vacía se rechaza: Active Directory la aceptaría como un
bind no autenticado sin comprobar nada.

## Opciones {#options}

| Opción | Por defecto | Qué |
|---|---|---|
| Connection security (`security`) | `ldaps` | `ldaps`, `starttls` o `plain` |
| Port (`port`) | 636, o 389 | se usa si la dirección no da uno |
| CA certificate (`ca_cert`) | | PEM de la autoridad que emitió los certificados de los controladores de dominio |
| Accept an unverifiable certificate (`insecure_tls`) | desactivado | |
| Allow plain LDAP (`allow_plaintext`) | desactivado | requerido para `plain` |
| Check every domain controller (`check_all_dcs`) | activado | conexión TCP al puerto LDAP de cada controlador de dominio |
| Inactive user after (`stale_days_users`) | 90 | días |
| Inactive computer after (`stale_days_computers`) | 90 | días |
| Full inventory every (`inventory_minutes`) | 15 | minutos, de 1 a 1440 |
| Timeout (`request_timeout_seconds`) | 8 | por operación LDAP, de 1 a 60 |

## Solución de problemas {#troubleshooting}

- **TLS certificate rejected**: pega el certificado de la autoridad
  como CA certificate, e introduce el controlador de dominio por un
  nombre que liste su certificado.
- **Bind refused: wrong user name or password (LDAP result 49)**: el
  código de diagnóstico del controlador de dominio se traduce —
  contraseña caducada, cuenta desactivada, bloqueada, o debe cambiar su
  contraseña en el próximo inicio de sesión.
- **The domain controller requires a protected connection**: la firma
  LDAP está forzada; cambia a LDAPS o StartTLS.
- **A domain controller shows as unresolved**: su nombre no resuelve
  desde el host de DumbMonit. Apunta el DNS de DumbMonit a los
  servidores DNS del dominio, o desmarca *Check every domain
  controller*. Los nombres no resueltos nunca generan una alerta.
- **Replication status not readable**: concede *Monitor active
  directory replication* como en el paso 3, o acepta que la
  replicación no se muestre.

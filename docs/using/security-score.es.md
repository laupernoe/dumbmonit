# Puntuación de seguridad {#security-score}

Cada dispositivo soportado obtiene una **puntuación de seguridad** de 0
a 100 y una nota de **A** a **F**, al estilo de PingCastle o Microsoft
Secure Score: una lista de comprobaciones extraídas de la propia guía
de buenas prácticas del fabricante, cada una aprobando, fallando o no
evaluada.

La puntuación aparece como una tarjeta **Security** en la página del
dispositivo, y en la página **Security**, que lista cada dispositivo
puntuado, el peor primero.

## Cómo se calcula la puntuación {#how-the-score-is-computed}

DumbMonit nunca pide nada extra al dispositivo para calcular la
puntuación. Las comprobaciones leen los últimos valores que ya
registraron los recolectores (mirando hasta 6 horas atrás), así que una
puntuación no cuesta nada al dispositivo y funciona sin conexión desde
la última sonda.

Cada comprobación tiene:

| Campo | Significado |
|---|---|
| Categoría | exposición, parcheo, autenticación, cifrado, copia de seguridad o configuración |
| Severidad | critical, high, medium o low — su peso en la puntuación: 10, 6, 3, 1 |
| Resultado | **pass**, **fail**, o **unknown** cuando no se recolectan los datos |
| Evidencia | lo que se observó: `pve1: 3 security updates`, `Expires in 9 days`… |
| Remediación | qué hacer, en los términos del fabricante |
| Guía del fabricante | la página del fabricante (o estándar) en la que se basa la comprobación |

**Puntuación** = 100 × (peso de las comprobaciones que pasan) ÷ (peso de
las que pasan + las que fallan). **Las comprobaciones desconocidas se
muestran pero nunca se cuentan**, ni a favor ni en contra: una
puntuación no debe bajar porque a un token de monitorización le falta
un privilegio, ni subir porque no se sabe nada. Un dispositivo cuyas
comprobaciones son todas desconocidas *no está puntuado*.

| Nota | Puntuación | Palabra |
|---|---|---|
| A | 90–100 | Strong |
| B | 75–89 | Good |
| C | 60–74 | Fair |
| D | 40–59 | Weak |
| F | 0–39 | Poor |

**Una comprobación crítica fallida limita la nota a C**, sea cual sea
la puntuación: un cortafuegos que ya no filtra, o un firmware
activamente explotado y sin parchear, no se compensa con diez ajustes
menores correctos. La tarjeta lo indica cuando ocurre.

## Historial y alertas {#history-and-alerts}

El servidor recalcula cada puntuación cada 15 minutos y la guarda como
`dumbmonit_security_score{target="<id>"}` (y el número de comprobaciones
fallidas como `dumbmonit_security_checks_failed`), así que puedes
graficarla y alertar sobre ella como cualquier otra métrica. Dos reglas
integradas la usan:

- **Security score dropped** — la puntuación cayó 10 puntos o más
  comparada con su mejor valor de las últimas 24 horas: algo nuevo está
  fallando.
- **Security score low** — la puntuación está por debajo de 40 (nota
  F). Edita la regla para mover el umbral a tu propio listón.

Consulta [Reglas de alerta](../alerting/rules.md#security-score).

## Comprobaciones por tipo {#checks-per-kind}

### Proxmox VE

| Comprobación | Severidad | Datos |
|---|---|---|
| Security updates installed | high | actualizaciones de seguridad pendientes por nodo (necesita `Sys.Modify` en el token, de lo contrario desconocido) |
| Proxmox VE packages up to date | medium | paquetes Proxmox actualizables por nodo |
| Running the installed kernel | medium | kernel más nuevo instalado que el que se ejecuta |
| No test repository in use | medium | repositorios estándar activados |
| Package repositories correctly configured | medium | errores y avisos que Proxmox VE lanza sobre los repositorios (p. ej. repositorio enterprise sin suscripción) |
| Enterprise repository subscription | low | estado de suscripción por nodo |
| Web interface certificate valid for 14+ days | medium | caducidad del certificado del nodo |
| Every guest is in a backup job | high | invitados no seleccionados por ningún job de copia |
| Backup jobs succeed | high | último resultado de cada job de copia |

Fuentes: [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Certificate Management](https://pve.proxmox.com/wiki/Certificate_Management),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server

| Comprobación | Severidad | Datos |
|---|---|---|
| System updates installed | high | actualizaciones pendientes (necesita `Sys.Modify` en `/system`) |
| Running the installed Backup Server version | medium | demonios que todavía ejecutan la versión previa a la actualización |
| Verify job scheduled | high | número de jobs de verificación |
| Prune job scheduled | medium | número de jobs de prune |
| Backups copied elsewhere (sync or tape) | medium | jobs de copia sync y de cinta (regla 3-2-1) |
| Garbage collection clean, no bad chunks | high | último resultado de GC y chunks malos por datastore |
| Certificate valid for 14+ days | medium | caducidad del certificado |

Fuentes: [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html),
[Package repositories](https://pbs.proxmox.com/docs/package-repositories.html),
[Certificate management](https://pbs.proxmox.com/docs/certificate-management.html).

### Synology DSM

| Comprobación | Severidad | Datos |
|---|---|---|
| DSM 7 or later | high | versión de DSM |
| Hyper Backup task configured | medium | número de tareas Hyper Backup |
| Hyper Backup tasks succeed | high | último resultado de cada tarea |

Fuentes: [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense y pfSense

| Comprobación | Severidad | Datos |
|---|---|---|
| OPNsense: firmware updates installed | high | actualizaciones de paquete pendientes (desconocido hasta que OPNsense haya comprobado actualizaciones) |
| OPNsense: on the current major release series | medium | nueva serie mayor disponible |
| OPNsense: no reboot pending after an update | medium | reinicio requerido |
| OPNsense: packet filter enabled | **critical** | estado de `pf` |
| pfSense: REST API package up to date | low | actualización del paquete de API REST disponible |

Fuentes: [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html),
[pfSense REST API](https://pfrest.org/).

### UniFi

| Comprobación | Severidad | Datos |
|---|---|---|
| Device firmware up to date | high | bandera de actualizable de cada dispositivo adoptado |

Fuente: [UniFi releases](https://community.ui.com/releases).

### Monitores TLS y HTTPS

Para monitores `tls`, `http`, `smtp`, `mqtt` y `websocket` que negocian
TLS. Un monitor de texto plano no tiene tarjeta Security.

| Comprobación | Severidad | Datos |
|---|---|---|
| Certificate valid for 14+ days | high | días hasta la caducidad |
| Certificate chain trusted | high | verificación de la cadena |
| TLS 1.3 negotiated | low | versión de protocolo negociada |

Fuentes: [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[NIST SP 800-52 Rev. 2](https://csrc.nist.gov/pubs/sp/800/52/r2/final),
[Let's Encrypt integration guide](https://letsencrypt.org/docs/integration-guide/).

### Servidores de hora NTP

| Comprobación | Severidad | Datos |
|---|---|---|
| Server reports a synchronized clock | medium | stratum e indicador leap |
| Clock offset within 100 ms | low | desfase medido |

Un stratum de 16 (o 0, un «kiss-o'-death») y un indicador leap
«unsynchronized» hacen fallar la primera comprobación sin importar el
desfase: un servidor que nunca se ha sincronizado no tiene nada fiable
que ofrecer. El umbral de 100 ms está fijo aquí y no sigue la opción
«Tolerated offset» por dispositivo, igual que la comprobación de
certificado siempre avisa a los catorce días sin importar los propios
ajustes de un monitor.

Fuente: [RFC 5905](https://www.rfc-editor.org/rfc/rfc5905).

### Nginx y Apache httpd

| Comprobación | Severidad | Datos |
|---|---|---|
| Server header does not disclose the version | low | la cabecera de respuesta `Server` capturada junto a la página de estado |

Una cabecera que todavía nombra el producto («nginx», «Apache») sin un
número de versión pasa: eso es lo que producen `server_tokens off` y
`ServerTokens Prod`, y una cabecera ausente también pasa. Una cabecera
como «nginx/1.25.3» o «Apache/2.4.58 (Unix)» falla: le dice a un escáner
exactamente qué CVEs probar.

Fuentes: [Nginx core module reference](https://nginx.org/en/docs/http/ngx_http_core_module.html#server_tokens),
[Apache core module reference](https://httpd.apache.org/docs/2.4/mod/core.html#servertokens).

### Máquinas con el agente (y sus contenedores Docker)

| Comprobación | Severidad | Datos |
|---|---|---|
| Security updates installed | high | actualizaciones de seguridad pendientes (donde el gestor de paquetes las distingue) |
| No pending updates | low | actualizaciones pendientes |
| No reboot pending | medium | reinicio requerido |
| SELinux enforcing | medium | modo SELinux (desconocido donde SELinux está ausente) |
| Container images up to date | medium | imagen más nueva en el registro (solo en máquinas con contenedores) |
| No container image older than 180 days | low | antigüedad de la imagen (solo en máquinas con contenedores) |

Fuentes: [CIS Control 7 — Continuous Vulnerability Management](https://www.cisecurity.org/controls/continuous-vulnerability-management),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index),
[Docker build best practices](https://docs.docker.com/build/building/best-practices/#rebuild-your-images-often).

### Actualizaciones de fabricante para otras integraciones

Una comprobación de parcheo por producto, a partir de la información de
actualización que ya lee cada recolector:

| Tipo | Comprobación | Severidad |
|---|---|---|
| FortiGate | FortiOS patch installed | **critical** |
| FortiGate | FortiGuard licences valid | high |
| MikroTik | RouterOS up to date | high |
| MikroTik | RouterBOOT firmware matches RouterOS | low |
| Home Assistant | Home Assistant components up to date | medium |
| Nextcloud | Nextcloud server up to date | high |
| Nextcloud | Nextcloud apps up to date | medium |
| AdGuard Home | AdGuard Home up to date | medium |
| Pi-hole | Pi-hole up to date | medium |
| Plex | Plex Media Server up to date | medium |
| Paperless-ngx | Paperless-ngx up to date | medium |
| Tailscale | Tailscale clients up to date | medium |

### Active Directory

Un dispositivo Active Directory se puntúa a partir de los hallazgos al
estilo PingCastle de su recolector: cuentas vulnerables a
Kerberoasting, cuentas sin preautenticación Kerberos, delegación sin
restricción, antigüedad de la contraseña `krbtgt`, tamaño de grupo
privilegiado, cobertura LAPS, política de contraseñas, cuentas
inactivas. Cada hallazgo es una comprobación que pasa cuando concierne a
cero objetos; la severidad que reporta el recolector es la que se usa.
Los hallazgos aún no reportados se listan como desconocidos.

Fuentes: [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Microsoft — unconstrained Kerberos delegation](https://learn.microsoft.com/en-us/defender-for-identity/security-assessment-unconstrained-kerberos),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview),
[Resetting the krbtgt password](https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/forest-recovery-guide/ad-forest-recovery-resetting-the-krbtgt-password).

### GitLab

Leído desde la API de administración, con un token de acceso personal
de administrador; desconocido con uno corriente.

| Comprobación | Severidad | Datos |
|---|---|---|
| Two-factor authentication required | high | ajuste `require_two_factor_authentication` |
| Public sign-up disabled | medium | ajuste `signup_enabled` |
| Database migrations applied | medium | migraciones pendientes tras una actualización |

Fuente: [GitLab application settings API](https://docs.gitlab.com/ee/api/settings.html).

### Forgejo y Gitea

| Comprobación | Severidad | Datos |
|---|---|---|
| Scheduled tasks run on time | medium | tareas cuya próxima ejecución ya está vencida |

Desconocido sin un token de acceso de administrador del sitio: Forgejo
y Gitea reservan la lista de tareas programadas a la administración.

Fuente: [Forgejo API](https://forgejo.org/docs/latest/user/api/).

## Límites {#limits}

La puntuación solo refleja lo que DumbMonit puede ver. Es una invitación
a mirar, no una auditoría, y una nota A no significa que un dispositivo
sea seguro.

Buenas prácticas que **todavía no** se comprueban porque los
recolectores no leen esos datos hoy:

| Tipo | No comprobado (necesita datos nuevos) |
|---|---|
| Proxmox VE | cortafuegos de datacenter/nodo activado; autenticación de doble factor en `root@pam` y otras cuentas de administrador |
| Proxmox Backup Server | cifrado del lado del cliente de las copias de seguridad (necesita leer los manifiestos de copia) |
| Synology DSM | verificación en 2 pasos para administradores; cuenta `admin` por defecto desactivada; bloqueo automático; cortafuegos DSM; redirección HTTP a HTTPS; exposición de QuickConnect; si hay disponible una actualización de DSM más reciente |
| OPNsense / pfSense | GUI web o SSH accesible desde la WAN; credenciales por defecto; actualizaciones del sistema pfSense (no de la API REST) |
| UniFi | SSH de dispositivo con credenciales por defecto; ajustes del sitio |
| Monitores TLS | TLS 1.0/1.1 o suites de cifrado débiles todavía ofrecidas — la sonda solo negocia TLS 1.2 y 1.3, así que un servidor limitado a versiones más antiguas se muestra como una sonda fallida en lugar de esto |
| Hosts de agente | estado del cortafuegos del host; configuración SSH; estado de Windows Defender |
| Docker | contenedores ejecutándose como privileged, como root, o con el socket de Docker montado |

Otros límites:

- Una comprobación es tan fresca como sus datos: algunos recolectores
  vuelven a leer actualizaciones o suscripciones cada pocas horas.
- Los pesos son una decisión de producto, la misma para cada
  instalación.
- Los tipos sin ninguna comprobación no tienen tarjeta Security y no
  aparecen en la página Security.

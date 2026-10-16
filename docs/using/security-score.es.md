# Puntuación de seguridad

La puntuación de seguridad es una nota **opcional** de la **A** a la **F** (y una
puntuación de 0 a 100) que se muestra junto a los monitores en los que unas
pocas comprobaciones dicen algo útil: hosts Linux y Windows con el agente,
Proxmox VE y Proxmox Backup Server, Synology, OPNsense, sitios web y Active
Directory. Los demás monitores no muestran nada.

!!! warning "Orientativa, no una auditoría"
    La puntuación es **aproximada y no exhaustiva**. Solo lee lo que DumbMonit
    ya recopila de un monitor y lo compara con un puñado de buenas
    prácticas del fabricante. Una nota A no significa que un monitor sea
    seguro, y una comprobación ausente no significa que el ajuste esté bien.
    Úsela como una invitación a mirar y, cuando importe, recurra a una
    herramienta de evaluación de verdad (PingCastle, Mozilla HTTP Observatory,
    Lynis, el asesor de seguridad del propio fabricante…).

## Activarla

La puntuación está **desactivada por defecto**. Un administrador la activa en
**Ajustes → Cuenta y seguridad → Puntuación de seguridad**. Una vez activada:

- cada monitor puntuado recibe una pequeña **insignia con forma de escudo**
  con su letra en la lista de monitores, y una insignia **Security** con la
  puntuación en su página;
- al hacer clic en la insignia se abre el **informe** del monitor: lo que
  falla, con la evidencia y la solución, lo que pasa, los pasos para mejorar la
  puntuación en este tipo de monitor y un enlace a la sección
  correspondiente más abajo.

Al desactivarla se ocultan las insignias, se detiene el cálculo periódico y
dejan de escribirse las series `dumbmonit_security_score`.

!!! note "Actualización desde 0.1.0-alpha.8 o anterior"
    La puntuación estaba antes siempre activa, con su propia página
    **Security**. Una instalación que ya monitorizaba un monitor puntuable
    mantiene la puntuación activada tras la actualización; una instalación
    nueva empieza con ella desactivada. La antigua dirección `/security` abre
    ahora el ajuste.

## Cómo se calcula la puntuación

DumbMonit nunca pide nada adicional al monitor para calcular la
puntuación. Las comprobaciones leen los últimos valores que los colectores ya
registraron (mirando 6 horas atrás), por lo que una puntuación no le cuesta
nada al monitor.

Cada comprobación tiene una categoría (exposición, parches, autenticación,
cifrado, copia de seguridad, configuración), una **severidad** que es su peso
(critical 10, high 6, medium 3, low 1), un resultado (**correcta**,
**fallida** o **desconocida** cuando el dato no se recopila), la evidencia, la
solución y un enlace a la página del fabricante en la que se basa.

**Puntuación** = 100 × (peso de las comprobaciones correctas) ÷ (peso de las
correctas + fallidas). **Las comprobaciones desconocidas se muestran pero
nunca se cuentan**: una puntuación no debe bajar porque a una cuenta de
monitorización le falte un privilegio, ni subir porque no se sepa nada. Un
monitor cuyas comprobaciones son todas desconocidas *no se puntúa*.

| Nota | Puntuación | Palabra |
|---|---|---|
| A | 90–100 | Sólida |
| B | 75–89 | Buena |
| C | 60–74 | Aceptable |
| D | 40–59 | Débil |
| F | 0–39 | Mala |

**Una comprobación crítica fallida limita la nota a C**, sea cual sea la
puntuación.

## Historial y alertas

Mientras la puntuación está activada, el servidor la recalcula cada 15 minutos
y la guarda como `dumbmonit_security_score{target="<id>"}` (y el número de
comprobaciones fallidas como `dumbmonit_security_checks_failed`). La usan dos
reglas integradas: **Security score dropped** (10 puntos o más por debajo del
mejor valor de las últimas 24 horas) y **Security score low** (por debajo de
40). Véase [Reglas de alerta](../alerting/rules.md#security-score).

## Comprobaciones y cómo mejorarlas

### Hosts Linux { #linux-hosts }

Máquinas con el agente. Las comprobaciones de SSH y del cortafuegos necesitan
un agente de la misma versión que el servidor; con un agente más antiguo son
desconocidas.

| Comprobación | Severidad | Dato |
|---|---|---|
| Security updates installed | high | actualizaciones de seguridad pendientes (donde el gestor de paquetes las distingue) |
| No pending updates | low | actualizaciones pendientes |
| No reboot pending | medium | reinicio necesario |
| SSH root login with a password refused | high | `PermitRootLogin` de `sshd -T` (`prohibit-password` pasa) |
| SSH accepts keys only | medium | `PasswordAuthentication`, y `KbdInteractiveAuthentication` con `UsePAM` |
| Host firewall filters inbound traffic | high | una cadena input de nftables o iptables que descarta o rechaza |
| SELinux enforcing | medium | modo de SELinux (desconocido donde SELinux no existe) |
| Container images up to date | medium | imagen más reciente en el registro (solo con contenedores) |
| No container image older than 180 days | low | antigüedad de la imagen (solo con contenedores) |

Para mejorarla:

1. Instale las actualizaciones de seguridad pendientes y active después las
   actualizaciones de seguridad automáticas: `unattended-upgrades` en Debian y
   Ubuntu, `dnf-automatic` en Fedora y RHEL.
2. Reinicie tras las actualizaciones del kernel o de bibliotecas.
3. Compruebe que su clave SSH funciona y luego establezca
   `PermitRootLogin no`, `PasswordAuthentication no` y
   `KbdInteractiveAuthentication no` en `/etc/ssh/sshd_config` y recargue
   sshd.
4. Active un cortafuegos de host que descarte el tráfico entrante no
   solicitado: `ufw enable` en Ubuntu, firewalld en Fedora y RHEL, un conjunto
   de reglas nftables en el resto.
5. Mantenga SELinux en enforcing donde la distribución lo incluya; descargue
   con regularidad imágenes de contenedor más recientes.

Fuentes: [CIS Controls](https://www.cisecurity.org/controls),
[sshd_config(5)](https://man.openbsd.org/sshd_config),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index).

### Hosts Windows { #windows-hosts }

Máquinas con el agente de Windows, de la misma versión que el servidor (se lee
del registro; un agente más antiguo no informa de ninguna de estas).

| Comprobación | Severidad | Dato |
|---|---|---|
| Windows Firewall on for every profile | high | perfiles de dominio, privado y público (primero la directiva de grupo) |
| No reboot pending | medium | Windows Update o mantenimiento de componentes esperando un reinicio |
| Remote Desktop requires Network Level Authentication | high | pasa cuando el Escritorio remoto está desactivado |
| SMBv1 disabled | high | ajuste del servidor SMBv1 |

Para mejorarla:

1. Instale las actualizaciones de Windows (Configuración → Windows Update) y
   reinicie cuando se le pida.
2. Mantenga activado el Firewall de Windows Defender en los perfiles de
   dominio, privado y público.
3. Si el Escritorio remoto está activado, exija la autenticación a nivel de red
   (NLA) y acceda a él por una VPN, nunca directamente desde internet.
4. Elimine SMBv1: `Disable-WindowsOptionalFeature -Online -FeatureName SMB1Protocol`.

Fuentes: [Windows Firewall](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/),
[SMBv1](https://learn.microsoft.com/en-us/windows-server/storage/file-server/troubleshoot/detect-enable-and-disable-smbv1-v2-v3).

### Proxmox VE { #proxmox-ve }

| Comprobación | Severidad | Dato |
|---|---|---|
| Security updates installed | high | actualizaciones de seguridad pendientes por nodo (necesita `Sys.Modify` en el token; si no, desconocida) |
| Proxmox VE packages up to date | medium | paquetes de Proxmox actualizables por nodo |
| Running the installed kernel | medium | kernel más reciente instalado que el que se ejecuta |
| No test repository in use | medium | repositorios estándar habilitados |
| Package repositories correctly configured | medium | errores y avisos que Proxmox VE genera sobre los repositorios |
| Enterprise repository subscription | low | estado de la suscripción por nodo |
| Web interface certificate valid for 14+ days | medium | caducidad del certificado del nodo |
| Every guest is in a backup job | high | invitados no seleccionados por ningún trabajo de copia |
| Backup jobs succeed | high | último resultado de cada trabajo de copia |

Para mejorarla:

1. Use un repositorio al que tenga derecho: enterprise con suscripción; si no,
   `pve-no-subscription`; nunca `pvetest` en un nodo que importe.
2. Instale las actualizaciones desde el panel **Updates** del nodo y reinicie
   con el nuevo kernel.
3. Ponga cada VM y contenedor en un trabajo de copia, idealmente hacia un
   Proxmox Backup Server, y corrija cualquier trabajo que falle.
4. Sustituya el certificado web autofirmado por uno ACME.

Fuentes: [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server { #proxmox-backup-server }

| Comprobación | Severidad | Dato |
|---|---|---|
| System updates installed | high | actualizaciones pendientes (necesita `Sys.Modify` en `/system`) |
| Running the installed Backup Server version | medium | demonios que aún ejecutan la versión anterior a la actualización |
| Verify job scheduled | high | número de trabajos de verificación |
| Prune job scheduled | medium | número de trabajos de prune |
| Backups copied elsewhere (sync or tape) | medium | trabajos de sincronización y de copia en cinta (regla 3-2-1) |
| Garbage collection clean, no bad chunks | high | último resultado del GC y chunks defectuosos por datastore |
| Certificate valid for 14+ days | medium | caducidad del certificado |

Para mejorarla:

1. Instale las actualizaciones y reinicie cuando haya cambiado el kernel o
   `proxmox-backup`.
2. Programe un trabajo de verificación y uno de prune en cada datastore.
3. Mantenga una copia en otro sitio: un trabajo de sincronización hacia un
   segundo Backup Server, o cinta.
4. Use un certificado ACME para la interfaz web.

Fuentes: [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html).

### Synology { #synology }

| Comprobación | Severidad | Dato |
|---|---|---|
| DSM 7 or later | high | versión de DSM |
| DSM up to date | high | hay una actualización de DSM disponible (solo cuenta de administrador) |
| Built-in admin account disabled | medium | estado de la cuenta `admin` (solo cuenta de administrador) |
| Auto Block enabled | medium | ajuste de Auto Block (solo cuenta de administrador) |
| DSM firewall enabled | medium | ajuste del cortafuegos (solo cuenta de administrador) |
| Hyper Backup task configured | medium | número de tareas de Hyper Backup |
| Hyper Backup tasks succeed | high | último resultado de cada tarea |

Los cuatro ajustes se leen una vez por hora y solo cuando DumbMonit inicia
sesión con una cuenta de administrador; con una cuenta normal siguen siendo
desconocidos y no cuentan. La verificación en dos pasos y la redirección a
HTTPS no se leen.

Para mejorarla:

1. Ejecute la última versión de DSM 7 para su modelo y deje que DSM instale
   automáticamente las actualizaciones importantes (Panel de control →
   Actualización y restauración).
2. Desactive la cuenta integrada `admin`; inicie sesión con un administrador
   personal protegido con verificación en dos pasos.
3. Active Auto Block y el cortafuegos (Panel de control → Seguridad), y no
   redirija puertos de DSM a internet: use una VPN.
4. Haga copias con Hyper Backup en otro monitor o en un destino en la nube,
   y corrija cualquier tarea que falle: RAID y las instantáneas no son copias
   de seguridad.

Fuentes: [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense { #opnsense }

| Comprobación | Severidad | Dato |
|---|---|---|
| Firmware updates installed | high | actualizaciones de paquetes pendientes (desconocida hasta que OPNsense haya buscado actualizaciones) |
| On the current major release series | medium | hay una nueva serie mayor disponible |
| No reboot pending after an update | medium | reinicio necesario |
| Packet filter enabled | **critical** | estado de `pf` |

Para mejorarla:

1. Busque actualizaciones en System → Firmware → Status, instálelas y reinicie
   cuando se le pida.
2. Pase a la nueva versión mayor cuando salga.
3. Mantenga activado el filtro de paquetes.
4. Permita la interfaz web y SSH solo desde la LAN o una VPN, nunca desde la
   WAN.

Fuentes: [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html).

### Sitios web { #websites }

Para los monitores **HTTP(S)** y **TLS** que negocian TLS, y para los
monitores **Website health & SEO**. Un monitor HTTP sin cifrar no tiene
puntuación.

| Comprobación | Severidad | Dato |
|---|---|---|
| Certificate valid for 14+ days | high | días hasta la caducidad (monitores HTTP(S) y TLS) |
| Certificate chain trusted | high | verificación de la cadena (monitores HTTP(S) y TLS) |
| TLS 1.3 negotiated | low | versión del protocolo negociada (monitores HTTP(S) y TLS) |
| Served over HTTPS | high | dirección final tras las redirecciones (Website health) |
| Strict-Transport-Security header | medium | cabecera con un `max-age` mayor que 0 |
| X-Content-Type-Options: nosniff | low | valor de la cabecera |
| Protected against clickjacking | low | `X-Frame-Options` o `frame-ancestors` de la CSP |
| Content-Security-Policy header | low | una política aplicada (report-only no cuenta) |

Las cabeceras se leen solo en respuestas HTTPS.

Para mejorarla:

1. Sirva el sitio solo por HTTPS y redirija el HTTP sin cifrar a HTTPS.
2. Automatice la renovación del certificado (ACME, p. ej. Let's Encrypt) y
   sirva la cadena completa.
3. Añada cabeceras de seguridad en el servidor web o el proxy inverso:
   `Strict-Transport-Security`, `X-Content-Type-Options: nosniff` y una
   `Content-Security-Policy` con `frame-ancestors`.
4. Contraste con [Mozilla HTTP Observatory](https://developer.mozilla.org/en-US/observatory),
   que comprueba mucho más que esta puntuación.

Fuentes: [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[MDN — HTTP headers](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers).

### Active Directory { #active-directory }

Puntuado a partir de los hallazgos al estilo PingCastle del colector de Active
Directory: cuentas kerberoastable, cuentas sin preautenticación Kerberos,
delegación sin restricciones, antigüedad de la contraseña de `krbtgt`, tamaño
de los grupos privilegiados, cobertura de LAPS, política de contraseñas,
cuentas obsoletas. Cada hallazgo es una comprobación que pasa cuando afecta a
cero objetos.

Para mejorarla:

1. Corrija primero los hallazgos críticos y altos: son los caminos habituales
   hacia los Domain Admins.
2. Si la contraseña de `krbtgt` es antigua, restablézcala dos veces, con unas
   horas de diferencia, y elimine la delegación sin restricciones de los
   equipos que no sean controladores de dominio.
3. Pase los servicios que se ejecutan con cuentas de usuario con SPN a cuentas
   de servicio administradas de grupo (gMSA).
4. Ejecute [PingCastle](https://www.pingcastle.com/) para una evaluación
   completa.

Fuentes: [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview).

## Límites

- La puntuación solo refleja lo que DumbMonit puede ver; muchas buenas
  prácticas no se comprueban porque ningún colector lee el dato: cortafuegos y
  autenticación de dos factores de Proxmox, interfaz web de OPNsense expuesta en
  la WAN, contenedores Docker que se ejecutan con privilegios o con el socket
  de Docker montado, TLS 1.0/1.1 aún ofrecidos (la sonda solo negocia TLS 1.2 y
  1.3), y muchas más.
- Una comprobación es tan fresca como sus datos: algunos colectores releen las
  actualizaciones solo cada pocas horas.
- Los pesos son una decisión de producto, la misma para todas las
  instalaciones.
- Solo se puntúan los tipos indicados arriba. Los demás (conmutadores,
  impresoras, UniFi, pfSense…) no muestran insignia a propósito: un par de
  comprobaciones darían una nota que no significa nada.

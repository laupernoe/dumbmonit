# Añade tu primer monitor {#add-your-first-device}

Añadir cualquier cosa siempre funciona igual: elige un tipo de la lista,
un aviso a la derecha explica qué preparar en el monitor, y el
formulario solo muestra los campos que importan para ese tipo.

## Los tres primeros pasos {#the-three-first-steps}

En una instancia completamente nueva, el resumen no muestra un boletín
vacío: muestra una guía corta — **añade tu primer monitor**,
**conecta una forma de que te avisen**, **comprueba que llega un
mensaje**. Cada paso es un clic, y cada uno se pone en verde por su
cuenta. El tercero solo cuenta un mensaje que realmente salió de un
canal, así que crear un canal y nunca probarlo no lo marca.

La guía se guarda en la instancia, no en el navegador: omitir un paso, o
toda la guía, lo omite en cada navegador y para cada usuario.
Desaparece para siempre una vez resueltos los tres pasos, y nunca
aparece en una instancia que ya tenía monitores y un canal —
actualizar no le entrega un tutorial a un homelab ya establecido.

## Añadir un dispositivo SNMP {#add-an-snmp-device}

SNMP es el tipo más universal: casi todo el hardware de red lo habla.
Switches, routers, NAS, SAI e impresoras son los sospechosos habituales.

1. En el monitor, activa SNMP v2c y anota la comunidad de solo
   lectura (`public` por defecto en muchos monitores). Si el
   monitor filtra por dirección, permite la dirección del servidor
   DumbMonit.
2. En DumbMonit, abre **Devices** y haz clic en **Add a device** (o
   pulsa ++ctrl+k++ y escribe «add»).
3. Elige **SNMP device**. El selector se pliega en una fila y el
   formulario muestra los campos para SNMP.
4. Introduce un **name**, la **address** (IP o nombre de host) y la
   **community**. Para SNMP v3, cambia la credencial a *SNMP v3* y
   rellena el nombre de usuario y las frases de paso de autenticación y
   privacidad.
5. Haz clic en **Add device**.

![Añadir un dispositivo SNMP: el formulario a la izquierda, el aviso de configuración a la derecha](../assets/screenshots/add-device-light.png){ loading=lazy }

El perfil de recolección se detecta automáticamente a partir del
`sysObjectID` del monitor, en segundo plano, así que el formulario
nunca espera a un monitor inaccesible. Las interfaces, y en
servidores y NAS la CPU, memoria y almacenamiento, aparecen dentro de un
minuto.

!!! warning "Las comunidades viajan en texto plano"
    Una comunidad SNMP v1/v2c no está cifrada en la red. En una red
    compartida, prefiere SNMP v3, que autentica y cifra.

## Escanear una red {#scan-a-network}

En lugar de escribir monitores uno por uno, escanea un rango:

1. En **Add a device**, abre el panel **Scan my network**.
2. Introduce una red en notación CIDR, por ejemplo `192.168.1.0/24`
   (hasta 4096 direcciones por escaneo), y la comunidad a probar en
   cada dirección (`public` por defecto).
3. Cada dirección que responde se lista con su `sysName`, descripción y
   el perfil que se aplicaría. Los monitores ya en DumbMonit se
   marcan *Already added*, y todo lo nuevo se marca para ti.
4. Desmarca lo que no quieras, y luego haz clic en **Add *n*
   devices**. Se crean uno por uno, cada fila mostrando su propio
   resultado, así que un fallo nunca detiene al resto.

## Añadir un servidor Proxmox VE {#add-a-proxmox-ve-server}

El mismo flujo, con una credencial en dos campos en lugar de uno:

1. En Proxmox, crea un usuario reservado para monitorización, un rol de
   solo lectura y un token de API — los cuatro comandos `pveum` están
   en el aviso junto al formulario, cada uno con un botón de copiar, y
   en la [página de Proxmox VE](../devices/proxmox.md). Nunca reutilices
   la cuenta con la que inicias sesión.
2. En DumbMonit, **Add a device** → **Proxmox VE**.
3. Introduce un **name** y la **address** de cualquier nodo (puerto
   8006 por defecto).
4. Copia el **full-tokenid** del token (`dumbmonit@pve!monitor`) en
   **Token ID** y su **value** (el UUID) en **Secret**. ¿Pegaste por
   error toda la cadena `user@pve!name=secret` en Token ID? El
   formulario la separa y te lo indica.
5. Proxmox viene con un certificado autofirmado: marca **Accept an
   unverifiable certificate** bajo *More options* si la primera sonda se
   queja de ello.
6. Haz clic en **Add device**.

[Proxmox Backup Server](../devices/pbs.md) funciona igual con
`proxmox-backup-manager`; un [NAS Synology](../devices/synology.md)
necesita una cuenta DSM dedicada (nombre de usuario y contraseña); un
[servidor con el agente](../devices/agent.md) no necesita ninguna
credencial en absoluto, solo el token de inscripción mostrado una vez
guardado el monitor.

## El aviso de configuración {#the-setup-notice}

Cada tipo lleva su propio aviso, escrito por el servidor y mostrado
junto al formulario: pasos numerados a hacer en el monitor (crear
un usuario de solo lectura, crear un token de API, permitir la
dirección…), con la ruta exacta de menú en el producto y un botón de
copiar en cada comando o valor, y luego un escollo común y un enlace a
la documentación del fabricante cuando existe. Las páginas por tipo de
esta documentación llevan los mismos pasos palabra por palabra — un
test en el servidor los mantiene idénticos:
[SNMP](../devices/snmp.md), [Proxmox VE](../devices/proxmox.md),
[Proxmox Backup Server](../devices/pbs.md),
[Synology DSM](../devices/synology.md), [agente](../devices/agent.md),
[servicios](../devices/services.md).

El principio detrás de cada aviso: una cuenta dedicada de solo lectura,
nunca la que usas para iniciar sesión. Un secreto de monitorización
filtrado no debe poder cambiar nada.

## Campos comunes {#common-fields}

Sea cual sea el tipo, el formulario tiene:

| Campo | Significado |
|---|---|
| Name | Cómo aparece el monitor en las listas y alertas. |
| Address | IP, nombre de host, `host:puerto` o URL según el tipo; el placeholder muestra la forma esperada. |
| Credential | Las familias que acepta el tipo, cada una con sus propios campos: comunidad, usuario y frases de paso de SNMP v3, token de Proxmox (Token ID + Secret), nombre de usuario y contraseña. Los secretos tienen un ojo de mostrar/ocultar y los valores pegados se limpian de espacios sueltos, comillas y saltos de línea. Guardados cifrados, nunca devueltos por la API: al editar, los campos vacíos conservan lo guardado. |
| Check interval | Con qué frecuencia lee DumbMonit el monitor. Por defecto 60 s, mínimo 10 s. |
| Parent device | Si el padre se cae, las alertas de este monitor se suprimen en lugar de enviarse. Consulta [supresión por dependencia](../alerting/index.md#dependency-suppression). |
| Enabled | Un monitor en pausa no se comprueba y no genera ninguna alerta. |
| Tags | Etiquetas libres clave/valor, copiadas en cada serie como `tag_<key>`. Las opciones específicas del tipo también viven aquí, bajo **More options**. |

## Qué ocurre después {#what-happens-next}

- El monitor aparece en la página **Devices** como una placa
  frontal de rack: LED de estado, nombre, tipo, dirección, última
  visita, y un sparkline.
- Las mediciones se escriben en VictoriaMetrics cada 5 segundos
  (`DUMBMONIT_FLUSH_INTERVAL_SECS`), así que la primera gráfica aparece
  tras la primera sonda más unos segundos. **Probe now** en la página
  del monitor fuerza una sonda y reporta cuántas muestras y series
  produjo: es la principal herramienta de diagnóstico.
- Las [reglas de alerta integradas](../alerting/rules.md) se aplican de
  inmediato: monitor inaccesible, CPU saturada, disco casi lleno, y
  así. Nada que configurar.
- Para recibir notificaciones, añade un [canal de
  notificación](../notifications.md) en **Settings → Notifications & reports**. Las
  reglas integradas notifican a cada canal activado.

!!! tip "¿Sin datos tras un minuto?"
    Abre la página del monitor: un error de configuración (comunidad
    incorrecta, certificado rechazado, capacidad faltante) se muestra
    en rojo bajo la dirección, con el motivo. Consulta las [preguntas
    frecuentes](../faq.md#no-data-after-adding-a-device).

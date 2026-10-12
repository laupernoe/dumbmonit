# Copia de seguridad y restauración {#backup-and-restore}

Dos cosas distintas, para dos accidentes distintos.

| | Paquete cifrado | Copia de seguridad local programada |
|---|---|---|
| Qué contiene | La configuración: dispositivos y sus credenciales, reglas, canales, páginas de estado, cuentas, tokens. | Toda la base de datos, como un archivo, más `secret.key`. |
| Dónde va | Donde tú lo pongas — está cifrado con una frase de paso que eliges. | `/data/backups/`, dentro del mismo volumen. |
| De qué te salva | Un servidor perdido, un traslado a hardware nuevo, una instalación limpia. | El cambio que lamentas diez minutos después, una actualización que salió mal. |
| Qué no contiene | Historial de métricas, estado e historial de alertas, el registro de auditoría, sesiones. | Historial de métricas (`vm/`). |

Ninguno sustituye un archivo de todo el volumen si también quieres
recuperar las gráficas; eso está al final de esta página.

!!! danger "`/data/secret.key` es lo que descifra las credenciales de tus dispositivos"
    Las comunidades SNMP, los tokens de Proxmox, las contraseñas SMTP y
    las URL de heartbeat se guardan cifradas con una clave derivada del
    secreto de la instancia. En el primer arranque, DumbMonit la escribe
    en `/data/secret.key` (o la lee de `DUMBMONIT_SECRET` si defines
    una).

    **Una copia de la base de datos sin ese archivo restaura una
    instancia que no puede hablar con nada.** El servidor se da cuenta y
    se niega a arrancar con un mensaje explícito en lugar de fallar en
    cada sonda — pero las credenciales han desaparecido, y cada
    dispositivo hay que volver a introducirlo.

    El paquete cifrado es la única excepción: sus secretos se vuelven a
    cifrar con tu frase de paso, así que se restaura sobre una instancia
    nueva que tiene su propio secreto, totalmente nuevo.

## El paquete cifrado {#the-encrypted-bundle}

**Settings → Backup → Download the bundle.** Elige una frase de paso de
al menos 16 caracteres — tres palabras bastan — y confírmala. El archivo
que se descarga es JSON plano con un cuerpo cifrado:

```json
{
  "format": "dumbmonit-backup",
  "version": 1,
  "created_at": "2026-09-22 19:04:11",
  "source_version": "0.1.0-alpha.3",
  "summary": { "targets": 12, "channels": 2, "rules": 41 },
  "kdf": { "algorithm": "argon2id", "salt": "…", "m_cost": 32768, "t_cost": 3, "p_cost": 1 },
  "cipher": "aes-256-gcm",
  "payload": "…"
}
```

La cabecera se mantiene legible para que puedas saber qué es un archivo
antes de escribir nada. Todo lo demás — direcciones incluidas — está
dentro de `payload`, cifrado con AES-256-GCM bajo una clave derivada de
tu frase de paso por Argon2id. **No hay recuperación**: un paquete cuya
frase de paso se pierde se pierde.

!!! warning "El paquete contiene cada credencial de esta instancia"
    Cualquiera con el archivo y la frase de paso puede leer la
    contraseña de cada dispositivo que monitorizas. Guárdalo donde
    guardas las contraseñas.

### Qué contiene {#what-is-in-it}

| Sección | Contenido |
|---|---|
| `targets` | Dispositivos: nombre, tipo, dirección, intervalo, opciones, padre, agente relay, y la credencial. |
| `rules` | Reglas de alerta, incluidas las integradas que hayas reajustado o desactivado. |
| `rule_overrides` | Umbrales y exclusiones por dispositivo. |
| `channels` | Canales de notificación, sus ajustes, su política por canal y sus secretos. |
| `notify_policy` | La política de notificación global (agrupamiento, límite por hora, flapping, URL pública). |
| `silences` | Ventanas de mantenimiento. |
| `status_pages` | Páginas de estado públicas y los servicios que lista cada una. |
| `incidents` | Incidentes y anuncios de mantenimiento, con sus actualizaciones. |
| `users` | Cuentas: nombre de usuario, nombre mostrado, rol, enlace SSO, activado o no. |
| `agent_tokens` | Tokens de inscripción de agente, como hashes. Los agentes instalados siguen enviando tras una restauración. |
| `api_tokens` | Tokens de API, como hashes. Los tokens existentes siguen funcionando. |
| `push_monitors` | Tokens de heartbeat, así las URL que ya llaman tus tareas cron siguen siendo las mismas. |

Los dispositivos se emparejan por **tipo + dirección**, que es también
la clave única de la tabla: importar el mismo paquete dos veces nunca
crea una segunda copia de nada.

### Contraseñas de cuenta {#account-passwords}

Por defecto las cuentas se exportan **sin** su hash de contraseña,
secreto TOTP ni códigos de recuperación. Las cuentas restauradas
entonces existen con el nombre y rol correctos, pero nadie puede iniciar
sesión con ellas hasta que un administrador defina una contraseña — o
hasta que inicien sesión por SSO, lo cual no necesita nada del paquete.

Activa **Include account passwords and 2FA secrets** cuando estás
trasladando una instancia en lugar de copiar su configuración. Un hash
Argon2 en un archivo que viaja puede atacarse sin conexión durante todo
el tiempo que el atacante quiera; lo por defecto es lo seguro.

### Qué no contiene {#what-is-not-in-it}

El historial de métricas (vive en VictoriaMetrics, no en la base de
datos), el estado e historial de alertas, las referencias de anomalía,
el registro de auditoría, las sesiones abiertas, el registro y última
visita de los agentes, y las cachés de sonda de los paneles Proxmox y
Synology. Son observaciones, no ajustes: vuelven por su cuenta.

Y **no el secreto de la instancia**. Ese es el punto: el paquete lleva
sus secretos recifrados bajo tu frase de paso precisamente para que se
pueda restaurar en algún sitio que nunca ha visto `secret.key`.

### Restaurar sobre una instancia nueva {#restoring-onto-a-fresh-instance}

1. Inicia el contenedor nuevo y crea la primera cuenta de administrador
   en `/setup`, con el código de configuración impreso en sus logs.
   Esa cuenta es tuya; la restauración no la tocará.
2. **Settings → Backup → Restore a bundle.** Elige el archivo, escribe
   la frase de paso, y pulsa **Check this backup**.
3. Lee el informe. Indica, sección por sección, qué se **crearía**, qué
   se **actualizaría** porque difiere de lo que ya hay, y qué ya es
   idéntico — más una nota para cualquier cosa que no pudo ubicar, como
   una regla que apuntaba a un dispositivo que el paquete no contiene.
4. Pulsa **Restore for real**.

Tres reglas que la restauración nunca rompe:

- **Nada se elimina nunca.** Una restauración hace que exista lo que
  describe el paquete; cualquier cosa que la instancia tenga además
  permanece.
- **Nada se duplica nunca.** Cada elemento se empareja por su clave
  natural — tipo + dirección para un dispositivo, `uid` para una regla,
  nombre para un canal, slug para una página de estado. Restaurar el
  mismo paquete dos veces reporta todo como ya idéntico y no escribe
  nada.
- **Las cuentas existentes se dejan en paz.** Una cuenta cuyo nombre de
  usuario ya está aquí se omite, nunca se sobrescribe: un archivo no
  debe poder cambiar el rol o la contraseña del administrador que
  ejecuta la restauración.

Una simulación no es una simulación de la restauración, *es* la
restauración, ejecutada dentro de una transacción que se revierte al
final. El informe no puede estar equivocado sobre lo que haría
aplicarla.

Un paquete escrito por un DumbMonit más reciente se rechaza con una
frase que lo indica — actualiza primero, luego restaura. Un paquete que
ha sido modificado, o cuya frase de paso es incorrecta, también se
rechaza: AES-GCM no puede distinguir los dos casos, y tampoco puede el
mensaje.

### Desde la línea de comandos {#from-the-command-line}

Ambas rutas necesitan una **sesión** de administrador, no un token de
API: un token vive en un script, y un script no tiene por qué exportar
cada credencial de la instancia.

```bash
# Iniciar sesión
curl -s -c jar -X POST http://localhost:8080/api/auth/login \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"username":"admin","password":"…"}'

# Exportar
curl -s -b jar -X POST http://localhost:8080/api/backup \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"passphrase":"three words are enough"}' -o backup.json

# Simulación, luego aplicar
jq -n --slurpfile b backup.json \
  '{bundle: $b[0], passphrase: "three words are enough", apply: false}' > restore.json
curl -s -b jar -X POST http://localhost:8080/api/backup/restore \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d @restore.json | jq
```

## Copias de seguridad locales programadas {#scheduled-local-backups}

DumbMonit escribe una copia de su propia base de datos de forma
programada, por defecto **una vez al día, conservando las últimas
siete**, en `/data/backups/`:

```
/data/backups/dumbmonit-20260922-030000.db
/data/backups/dumbmonit-20260922-030000.key
```

El archivo `.db` se produce con `VACUUM INTO` de SQLite: una copia en
línea y consistente escrita mientras el servidor sigue funcionando, no
una copia de archivo que podría capturar la base de datos a mitad de
una escritura. `sqlite3` la abre tal cual, sin ningún journal que
reproducir. El `.key` a su lado es la copia de `secret.key` — juntos, el
par basta para reconstruir la instancia.

Viven en el mismo volumen que la base de datos, así que deshacen un
error, no un disco perdido. Cópialos fuera de la máquina si quieres más
que eso.

### Ajustes {#settings}

| Variable | Por defecto | Rol |
|---|---|---|
| `DUMBMONIT_BACKUP_ENABLED` | `1` | `0`, `false`, `no` u `off` para dejar de escribir copias programadas. |
| `DUMBMONIT_BACKUP_DIR` | `<directorio de datos>/backups` | Dónde se escriben. Apúntalo a un segundo volumen para sobrevivir a la pérdida del primero. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Horas entre dos copias de seguridad. La primera ocurre un intervalo después del arranque, no al arrancar. |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Cuántas se conservan. Las más antiguas se eliminan, con su `.key`. |

Nada más en el directorio se toca nunca: solo se rotan los archivos
llamados `dumbmonit-<timestamp>.db` y su `.key`.

Cuando el secreto de la instancia viene de `DUMBMONIT_SECRET` no hay
ningún archivo que copiar, y no se escribe ninguno — guarda ese valor en
tu gestor de contraseñas en su lugar. La sección Backup indica en cuál
de los dos casos estás.

### Saber que se ejecutó {#knowing-that-it-ran}

Cada ejecución publica tres series:

| Métrica | Significado |
|---|---|
| `dumbmonit_instance_backup_last_success_seconds` | Timestamp Unix de la última copia de seguridad **exitosa**. Un intento fallido nunca la refresca. |
| `dumbmonit_instance_backup_last_status` | `1` si el último intento tuvo éxito, `0` si falló. |
| `dumbmonit_instance_backup_size_bytes` | Tamaño del último archivo escrito. |

La regla integrada **DumbMonit backup did not run**
(`instance_backup_missing`) se activa cuando el último éxito tiene más
de dos días — una noche perdida puede ser un reinicio, dos es un
problema. La serie solo existe una vez escrita una primera copia de
seguridad, así que una instancia con la programación desactivada nunca
recibe el aviso.

**Settings → Backup** muestra lo mismo sin MetricsQL: la última
ejecución y si tuvo éxito, los archivos conservados y su tamaño, y
**Back up now** para escribir una de inmediato.

### Restaurar una {#restoring-one}

Detén el stack, pon el archivo de vuelta bajo el nombre de la base de
datos, y restaura la clave junto a él:

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data alpine sh -c '
  cp /data/backups/dumbmonit-20260922-030000.db /data/dumbmonit.db &&
  cp /data/backups/dumbmonit-20260922-030000.key /data/secret.key &&
  rm -f /data/dumbmonit.db-wal /data/dumbmonit.db-shm'
docker compose start
```

Eliminar los archivos `-wal` y `-shm` importa: pertenecen a la base de
datos que acabas de reemplazar, y SQLite intentaría aplicarlos a la
nueva.

Para leer una copia de seguridad sin restaurarla, cópiala fuera y ábrela
con `sqlite3`:

```bash
docker run --rm -v dumbmonit-data:/data -v "$PWD:/out" alpine \
  cp /data/backups/dumbmonit-20260922-030000.db /out/
sqlite3 dumbmonit-20260922-030000.db 'SELECT name, kind, address FROM targets;'
```

Las credenciales de ese archivo permanecen ilegibles sin el
`secret.key` correspondiente.

## Archivar todo el volumen {#archiving-the-whole-volume}

La única forma de conservar también las gráficas. Un volumen con
nombre, `dumbmonit-data`, contiene todo: la base de datos, el secreto,
las copias de seguridad locales y los datos de VictoriaMetrics.

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data -v "$PWD:/backup" alpine \
  tar czf /backup/dumbmonit-data.tgz -C /data .
docker compose start
```

Para restaurar, crea el volumen, extrae el archivo comprimido en él de
la misma forma, y luego `docker compose up -d`.

Si el espacio importa, `--exclude=./vm` mantiene el archivo pequeño: la
base de datos y el secreto son la configuración, `vm/` es solo el
historial.

## Qué hacer antes de una actualización {#what-to-do-before-an-upgrade}

1. **Settings → Backup → Back up now** — treinta segundos, e incluye el
   secreto.
2. Descarga también un paquete si la actualización es grande, o si te
   mudas a otra máquina.
3. `docker compose pull && docker compose up -d`. Las migraciones se
   ejecutan al arrancar.

Si algo sale mal, [restaura la copia de seguridad local](#restoring-one)
y arranca la etiqueta de imagen anterior.

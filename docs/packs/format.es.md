# Referencia del formato de pack {#pack-format-reference}

Un pack es un único archivo YAML, `pack.yaml`, en la versión de formato
1. Las claves desconocidas son errores, no ajustes ignorados: una
errata como `treshold` se reporta.

```text
my-pack/
  pack.yaml          el propio pack: el único archivo que necesita el servidor
  fixtures/          una respuesta capturada por fuente, nombrada según su id
    status.json
  expected.prom      lo que la extracción debe producir a partir de los fixtures
```

## Cabecera {#header}

| Clave | Obligatorio | Significado |
|---|---|---|
| `schema` | sí | Siempre `1`. |
| `id` | sí | Letras minúsculas, dígitos y guiones simples, empezando por una letra, hasta 40 caracteres: `shelly-plug`. El tipo de dispositivo se convierte en `pack.<id>`, el prefijo de métrica `dumbmonit_<id con guiones bajos>_`. |
| `version` | sí | [Versión semántica](https://semver.org) del pack: `1.2.0`. |
| `requires` | no | Versiones de DumbMonit con las que funciona el pack, como un requisito semver: `">=0.2"`. Un servidor fuera del rango rechaza el pack. |
| `label` | sí | Nombre mostrado en la lista de tipos de dispositivo. |
| `summary` | no | Una frase: qué vigila el pack. |
| `examples` | no | Productos a los que se aplica, mostrados como pistas. |
| `address_hint` | no | Placeholder del campo de dirección: `192.168.1.50`. |
| `default_port` | no | Puerto usado cuando la dirección no tiene uno. |
| `scheme` | no | `http` (por defecto) o `https`; el dispositivo puede anularlo. |
| `setup` | no | El aviso mostrado junto al formulario de dispositivo: `title`, `steps` (una lista; lo que sigue a un salto de línea en un paso se muestra como un comando copiable), `warning`, `doc_url`. |
| `credentials` | no | Tipos de credencial aceptados, en orden de preferencia: `none`, `api_token`, `username_password`. Por defecto: `[none]`. Una entrada puede ser `{kind, label, help}` para reformular el formulario. |
| `options` | no | Ajustes mostrados en el formulario de dispositivo (abajo). |

## Opciones {#options}

Las opciones se guardan en el dispositivo y se leen con
`{{option.<key>}}` en rutas, cabeceras y cuerpos, y en los umbrales de
regla.

```yaml
options:
  - key: switch_id          # minúsculas, dígitos, guiones bajos
    label: Switch channel
    help: 0 for a plug; 0, 1, 2… for each channel of a relay.
    input: number           # text (por defecto), number, boolean o select
    default: 0
    required: false
    placeholder: ""
    choices: []             # los valores de un select
```

Cada pack que sondea también obtiene cinco opciones de conexión, que no
puede redeclarar: `scheme`, `port`, `insecure_tls`,
`request_timeout_seconds` (1 a 120, por defecto 10) y
`allow_private_targets`.

## Fuentes {#sources}

Una fuente es una petición por sondeo, siempre enviada a la propia
dirección del dispositivo.

```yaml
sources:
  - id: status              # minúsculas, dígitos, guiones bajos
    type: http              # http o prometheus
    path: /rpc/Switch.GetStatus?id={{option.switch_id}}
    method: GET             # GET (por defecto) o POST
    format: json            # solo http: json (por defecto) o text
    headers:
      Accept: application/json
    body: '{"id": {{option.switch_id}}}'   # solo POST
    auth: auto              # auto (por defecto), none, bearer o basic
```

- `path` debe empezar con una única `/`. La URL de la petición es la
  dirección del dispositivo seguida de la ruta; tras la sustitución
  debe conservar el mismo esquema, host y puerto, o se rechaza la
  petición. Los valores de opción se codifican en porcentaje en las
  rutas, así que un valor no puede añadir un segmento ni cambiar el
  host.
- `auth: auto` envía una credencial `api_token` como
  `Authorization: Bearer …` y una credencial `username_password` como
  autenticación básica. `none` no envía nada implícitamente, así que
  una cabecera puede llevar la credencial en su lugar:
  `X-Api-Key: "{{credential.token}}"`. `{{credential.token}}`,
  `{{credential.username}}` y `{{credential.password}}` se aceptan
  **solo en cabeceras**: nunca en una ruta o un cuerpo, que terminan en
  los registros.
- Las redirecciones solo se siguen al mismo esquema, host y puerto, tres
  como máximo. `401` y `403` se reportan como errores de autenticación,
  otros estados no 2xx como errores de protocolo.
- Un sondeo hace como máximo 16 peticiones y lee como máximo 4 MiB por
  respuesta.

### Fuentes Prometheus {#prometheus-sources}

```yaml
sources:
  - id: metrics
    type: prometheus
    path: /metrics
    keep: [node_load1, node_memory_MemAvailable_bytes]
    rename:
      node_memory_MemAvailable_bytes: memory_available_bytes
    drop:
      fstype: ^(tmpfs|overlay)$
```

- `keep` lista las familias importadas tal cual, con todas sus
  etiquetas. Un nombre de familia también cubre sus series de
  histograma y summary (`_bucket`, `_sum`, `_count`). Los contadores,
  histogramas y sumas de summary siguen siendo contadores; todo lo
  demás es un gauge.
- `rename` le da a una familia conservada su nombre DumbMonit. Un
  nombre que no sea letras minúsculas, dígitos y guiones bajos debe
  renombrarse.
- `drop` descarta cada muestra cuya etiqueta coincida con la expresión
  regular.
- Las etiquetas que DumbMonit define él mismo (`target`, `host`,
  `instance`, `job`, `tag_*`…) se conservan como `exported_<nombre>`.

## Métricas {#metrics}

Cada métrica tiene un nombre *sin* prefijo: `power_watts` en el pack
`shelly-plug` se escribe `dumbmonit_shelly_plug_power_watts`. Los
nombres son letras minúsculas, dígitos y guiones bajos, y únicos dentro
del pack.

```yaml
metrics:
  - name: power_watts
    help: Active power drawn by the load.
    source: status
    kind: gauge             # gauge (por defecto) o counter
    json: $.apower          # exactamente uno de json, regex o prom
    scale: 1                # multiplicador, p. ej. 0.001 para ms → s
    map: {on: 1, off: 0}    # valor de texto → número
    labels: {}
```

| Extractor | Fuente | Valor | `labels` |
|---|---|---|---|
| `json: <JSONPath>` | `http`, formato `json` | Cada nodo que coincide con la ruta ([RFC 9535](https://www.rfc-editor.org/rfc/rfc9535) JSONPath, wildcards y filtros incluidos) genera una serie. | etiqueta → JSONPath evaluado en el objeto que *contiene* el valor: `$.sensors[*].value` con `sensor: $.name` etiqueta cada valor con su `name` hermano. |
| `regex: <expression>` | `http`, formato `text` | Cada coincidencia genera una serie; el valor es el grupo llamado `value`, si no el primer grupo. | etiqueta → nombre de un grupo de captura. |
| `prom: <sample name>` | `prometheus` | Cada muestra con este nombre exacto. | etiqueta → etiqueta de fuente. Vacío: se conservan todas las etiquetas de fuente. |

Valores: los números se toman tal cual; `true`/`false` se convierten en
1/0; el texto se busca en `map`, o se analiza como un número cuando no
hay `map`. Un valor que falta, es `null`, o está fuera de `map` no
produce nada para este sondeo.

## Discover {#discover}

`discover` convierte cada elemento de una lista en un conjunto de
series que comparten etiquetas: una fila por disco, sensor o cola.

```yaml
discover:
  - name: disks
    source: list
    rows: $.disks[*]        # JSONPath: una fila por coincidencia
    max_rows: 100           # por defecto 100, como máximo 1000
    labels:                 # JSONPath evaluado en cada fila
      disk: $.name
      model: $.info.model
    metrics:
      - name: disk_used_bytes
        json: $.used        # JSONPath evaluado en cada fila
      - name: disk_healthy
        json: $.state
        map: {ok: 1, degraded: 0}
```

Las métricas de fila aceptan `name`, `help`, `kind`, `json`, `scale` y
`map`. Las filas más allá de `max_rows` se ignoran.

## Reglas {#rules}

Las reglas se instalan con el pack, como reglas de alerta normales con
el uid `pack:<id>:<name>`. Entonces te pertenecen: una actualización del
pack añade las reglas que todavía no tiene, y nunca reescribe una que
cambiaste o eliminaste.

```yaml
rules:
  - name: overheating       # minúsculas, dígitos, guiones bajos
    title: Shelly plug overheating
    description: The internal temperature stayed above the limit for five minutes.
    expr: dumbmonit_shelly_plug_temperature_celsius
    op: ">"                 # >, >=, < o <=
    threshold: "{{option.max_temperature}}"   # o un número
    for: 5m                 # 30s, 5m, 1h, 1d
    severity: warning       # info, warning (por defecto) o critical
    unit: °C
```

- `expr` es [MetricsQL](https://docs.victoriametrics.com/metricsql/) y
  debe usar al menos una métrica del pack, por su nombre completo.
  Cualquier otro nombre `dumbmonit_*` que cite también debe ser
  producido por el pack (`dumbmonit_up` excepto): un pack no puede
  alertar sobre la serie de otro tipo.
- Un `threshold` de `{{option.x}}` instala el valor por defecto de la
  opción, y el propio valor de esa opción en cada dispositivo se
  convierte en su umbral para esta regla (una anulación por
  dispositivo, reescrita cada vez que se guarda el dispositivo).

## Perfiles SNMP {#snmp-profiles}

Un pack puede traer perfiles SNMP, escritos exactamente como los
integrados en
[`profiles/`](https://github.com/laupernoe/dumbmonit/tree/main/profiles).
Se unen al catálogo SNMP y se aplican por `sysObjectID` como los demás;
no crean un tipo de dispositivo propio.

```yaml
snmp_profiles:
  - id: acme-ups            # el id del pack, o empezando con "<id del pack>-"
    name: ACME UPS
    match:
      sysobjectid: [1.3.6.1.4.1.99999]
    include: [system]
    metrics:
      - name: acme_ups_battery_percent   # debe empezar con el prefijo del pack
        oid: 1.3.6.1.4.1.99999.1.1.0
        kind: gauge
```

El catálogo SNMP se lee al arrancar: reinicia DumbMonit tras instalar o
actualizar un pack con perfiles SNMP.

## Límites {#limits}

| Límite | Valor |
|---|---|
| Tamaño del pack | 256 KiB |
| Fuentes (peticiones por sondeo, redirecciones incluidas) | 16 |
| Tamaño de respuesta | 4 MiB |
| Series por sondeo | 5000 (el resto se descarta y se registra) |
| Métricas declaradas | 256 |
| Reglas | 64 |
| Filas por `discover` | 1000 |
| Longitud de valor de etiqueta | 200 bytes |

## Fixtures y salida esperada {#fixtures-and-expected-output}

`dumbmonit pack test <dir>` lee `fixtures/<id de fuente>.json`, `.txt` o
`.prom`, ejecuta la misma extracción que el servidor, y compara el
resultado con `expected.prom`: el formato de texto de Prometheus,
ordenado, sin timestamps y sin las etiquetas de dispositivo (`target`,
`host`, `tag_*`) que añade el servidor. `--update` reescribe
`expected.prom` a partir de los fixtures; revisa el diff antes de
confirmarlo.

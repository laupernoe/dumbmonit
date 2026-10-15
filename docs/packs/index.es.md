# Packs de integración {#integration-packs}

Un pack de integración añade un tipo de monitor a DumbMonit sin una
versión nueva. Es un único archivo YAML que dice dónde leer (una API
HTTP o una página Prometheus `/metrics` en el monitor), qué extraer,
y qué reglas de alerta vienen con él. Una vez instalado, el tipo del
pack aparece en **Add device** junto a los integrados, y sus
monitores se sondean, grafican y alertan como cualquier otro.

Un pack es declarativo: sin código, sin scripts. DumbMonit ejecuta cada
petición él mismo, bajo reglas que el pack no puede cambiar.

## Qué puede y no puede hacer un pack {#what-a-pack-can-and-cannot-do}

| Un pack puede | Un pack no puede |
|---|---|
| Llamar a rutas `GET` o `POST` en la propia dirección del monitor | Llamar a ningún otro host, seguir una redirección a otro host, o usar una URL absoluta |
| Leer JSON (JSONPath), texto plano (expresiones regulares) o el formato de texto de Prometheus | Ejecutar código, comandos de shell o plantillas más allá de `{{option.x}}` y `{{credential.x}}` |
| Enviar la credencial del monitor en una cabecera o como autenticación basic/bearer | Poner una credencial en una URL o un cuerpo de petición |
| Declarar opciones mostradas en el formulario de monitor (un canal de switch, un umbral) | Escribir métricas fuera de su propio prefijo `dumbmonit_<pack>_` |
| Incluir reglas de alerta sobre sus propias métricas | Cambiar un tipo de monitor integrado o una regla integrada |
| Traer perfiles SNMP, en el formato de los integrados | Leer más de 4 MiB por respuesta, hacer más de 16 peticiones por sondeo, o producir más de 5000 series por sondeo |

Las direcciones loopback y link-local (`127.0.0.1`, `169.254.x.x`) se
rechazan a menos que la opción **Allow loopback and link-local
targets** del monitor esté activada, exactamente como las
comprobaciones HTTP. Las direcciones de LAN privada siempre se
permiten.

## Packs de referencia {#reference-packs}

El repositorio incluye tres packs bajo
[`packs/`](https://github.com/laupernoe/dumbmonit/tree/main/packs), cada
uno con sus fixtures y salida esperada:

| Pack | Lee | Reglas |
|---|---|---|
| `node-exporter` | un subconjunto curado de Prometheus node_exporter (CPU, memoria, sistemas de archivos, discos, red) | sistema de archivos casi lleno, memoria casi agotada |
| `shelly-plug` | enchufes y relés Shelly Gen2+, `/rpc/Switch.GetStatus` | sobrecalentamiento |
| `speedtest-tracker` | el último resultado de la API v1 de Speedtest Tracker | internet por debajo de lo esperado, prueba de velocidad fallando |

## Instalar un pack {#install-a-pack}

En la interfaz, como administrador: **Settings → Integration packs**.
Pega el YAML del pack o sube su `pack.yaml`, y luego **Install the
pack**. Un pack que no pasa las comprobaciones se rechaza con la lista
de qué corregir; uno que pasa se guarda con sus notas, si las hay. La
misma página activa y desactiva packs y los desinstala. Los tipos
instalados aparecen bajo **Community packs** al añadir un monitor,
marcados *Pack*.

A través de la API HTTP, necesitas una sesión de administrador o un
token de API con el alcance `write` (Settings → **API & assistants**).

```bash
# Instalar, o actualizar a una versión más reciente del mismo pack
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @packs/shelly-plug/pack.yaml http://localhost:8080/api/packs

# Listar lo que está instalado
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/api/packs
```

La respuesta indica qué ocurrió (`created`, `updated` o `unchanged`),
las métricas que produce el pack y cuántas reglas de alerta se
añadieron. Un pack que no pasa la validación se rechaza con la lista
completa de problemas, uno por línea.

| Llamada | Hace |
|---|---|
| `GET /api/packs` | packs instalados: versión, tipo, métricas, reglas, monitores que lo usan |
| `GET /api/packs/{id}` | un pack |
| `POST /api/packs` | instala o actualiza; cuerpo: el YAML (`application/yaml`), o `{"yaml": "…"}` en JSON |
| `PUT /api/packs/{id}/disable` | detiene el sondeo de los monitores del pack, conserva todo lo demás |
| `PUT /api/packs/{id}/enable` | vuelve a sondearlos |
| `DELETE /api/packs/{id}` | desinstala, con sus reglas; rechazado (`409`) mientras monitores todavía lo usen |

Con una cookie de sesión en lugar de un token, añade `X-Requested-With:
DumbMonit` a cada `POST`, `PUT` y `DELETE`, como para el resto de la
[API](../reference/api.md).

Instalar surte efecto de inmediato: el tipo nuevo está disponible en el
formulario de monitor sin reiniciar. La única excepción es un pack
que trae perfiles SNMP; la respuesta entonces dice
`"restart_required": true`, porque el catálogo SNMP se lee al arrancar.

### Actualizaciones y tus cambios {#updates-and-your-changes}

Actualizar un pack (mismo `id`, contenido nuevo) sustituye su extracción
y su descripción. Sus reglas de alerta solo se *añaden*: una regla que
cambiaste, desactivaste o eliminaste nunca se reescribe por una
actualización.

Desactivar un pack retira su tipo de monitor y conserva todo lo
demás: monitores, historial, reglas. Sus monitores ya no se
sondean (cada uno muestra un error de configuración) y, como cualquier
monitor que deja de reportar, generan *Monitor unreachable* tras
tres minutos: pon también en pausa esos monitores si desactivas el
pack por un tiempo.

### Copias de seguridad {#backups}

Los packs instalados viajan en las [copias de
seguridad](../install/backup.md) con su YAML, y se restauran antes que
los monitores que los usan.

## Comprobar un pack antes de instalarlo {#check-a-pack-before-installing-it}

El binario del servidor valida y prueba packs sin conexión:

```bash
# Comprobaciones de esquema y consistencia, sin un servidor ni una base de datos
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack lint /packs/shelly-plug

# Reproduce fixtures/ a través de la extracción real y compara con expected.prom
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /packs/shelly-plug
```

`pack test --update` reescribe `expected.prom` a partir de los
fixtures (monta el directorio en lectura-escritura y ejecuta con
`--user "$(id -u)"` para que el archivo siga siendo tuyo).

Siguiente: [escribe tu primer pack](first-pack.md) en unos diez
minutos, o lee la [referencia del formato](format.md).

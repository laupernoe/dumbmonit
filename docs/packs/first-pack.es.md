# Escribe tu primer pack {#write-your-first-pack}

En unos diez minutos, esta página construye el pack de referencia
`shelly-plug` desde cero: un enchufe inteligente Shelly (Plus, Pro o
Gen3) leído a través de su API local, con potencia, energía y
temperatura, y una alerta cuando se sobrecalienta.

Necesitas Docker, y o bien un enchufe Shelly en tu red o la respuesta
capturada abajo.

## 1. Mira qué responde el dispositivo {#1-look-at-what-the-device-answers}

Los dispositivos de segunda generación de Shelly describen un relé con
una llamada:

```bash
curl http://192.168.1.50/rpc/Switch.GetStatus?id=0
```

```json
{
  "id": 0,
  "source": "HTTP_in",
  "output": true,
  "apower": 8.9,
  "voltage": 237.5,
  "current": 0.069,
  "aenergy": { "total": 6532.2, "by_minute": [45.199, 47.141, 88.397] },
  "temperature": { "tC": 41.3, "tF": 106.3 }
}
```

Guarda esta respuesta como `shelly-plug/fixtures/status.json`: es el
fixture contra el que se probará el pack, así que puedes trabajar sin el
dispositivo a mano.

## 2. La cabecera {#2-the-header}

Crea `shelly-plug/pack.yaml`:

```yaml
schema: 1
id: shelly-plug
version: 1.0.0
label: Shelly plug (Gen2+)
summary: Power, energy, voltage and temperature of a Shelly Plus/Pro/Gen3 plug or relay.
examples: [Shelly Plus Plug S, Shelly Pro 1PM]
address_hint: 192.168.1.50
default_port: 80
credentials: [none]
```

El `id` decide dos nombres: el tipo de dispositivo `pack.shelly-plug`, y
el prefijo de métrica `dumbmonit_shelly_plug_`.

## 3. Una fuente {#3-one-source}

Una fuente es una petición, siempre a la propia dirección del
dispositivo:

```yaml
options:
  - key: switch_id
    label: Switch channel
    help: 0 for a plug; 0, 1, 2… for each channel of a multi-channel relay.
    input: number
    default: 0

sources:
  - id: status
    type: http
    path: /rpc/Switch.GetStatus?id={{option.switch_id}}
```

`{{option.switch_id}}` se sustituye por el propio ajuste del
dispositivo, mostrado en el formulario de dispositivo como **Switch
channel**. El nombre de archivo del fixture, `status.json`, coincide con
el `id` de la fuente.

## 4. Las métricas {#4-the-metrics}

Cada métrica toma un valor con una expresión
[JSONPath](https://www.rfc-editor.org/rfc/rfc9535). Los nombres se
escriben sin el prefijo:

```yaml
metrics:
  - name: output_on
    help: 1 when the relay is closed (power flows), 0 when it is open.
    source: status
    json: $.output
  - name: power_watts
    source: status
    json: $.apower
  - name: voltage_volts
    source: status
    json: $.voltage
  - name: current_amperes
    source: status
    json: $.current
  - name: energy_watthours_total
    source: status
    kind: counter
    json: $.aenergy.total
  - name: temperature_celsius
    source: status
    json: $.temperature.tC
```

`output` es un booleano: se convierte en 1 o 0. El total de energía solo
crece, así que es un `counter`: las gráficas y reglas lo leen con
`rate()`, y un reinicio del enchufe no parece una caída.

## 5. Compruébalo {#5-check-it}

```bash
docker run --rm -v "$PWD:/work" --user "$(id -u)" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /work/shelly-plug --update
```

```text
ok: shelly-plug 1.0.0 (pack.shelly-plug, 6 metrics, 0 rules)
  wrote /work/shelly-plug/expected.prom
```

`expected.prom` muestra exactamente lo que el servidor guardaría a
partir del fixture:

```text
# TYPE dumbmonit_shelly_plug_power_watts gauge
dumbmonit_shelly_plug_power_watts 8.9
# TYPE dumbmonit_shelly_plug_temperature_celsius gauge
dumbmonit_shelly_plug_temperature_celsius 41.3
…
```

Léelo una vez. A partir de ahora, `pack test` sin `--update` falla si un
cambio al pack cambia lo que extrae, e imprime las líneas que difieren.

## 6. Una regla de alerta {#6-an-alert-rule}

Las reglas vienen con el pack y usan sus métricas por su nombre
completo. El umbral puede venir de una opción, así que cada enchufe
puede tener el suyo:

```yaml
options:
  # … switch_id, then:
  - key: max_temperature
    label: Temperature alert (°C)
    input: number
    default: 70

rules:
  - name: overheating
    title: Shelly plug overheating
    description: The internal temperature stayed above the limit for five minutes.
    expr: dumbmonit_shelly_plug_temperature_celsius
    op: ">"
    threshold: "{{option.max_temperature}}"
    for: 5m
    severity: warning
    unit: °C
```

Ejecuta `pack lint` para comprobar todo el archivo: una regla que cita
una métrica que el pack no produce, una opción que no existe o una ruta
que apunta a otro lugar se reportan con dónde están.

```bash
docker run --rm -v "$PWD:/work:ro" ghcr.io/laupernoe/dumbmonit:latest pack lint /work/shelly-plug
```

## 7. Instálalo {#7-install-it}

Con un token de API que tenga el alcance `write`:

```bash
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @shelly-plug/pack.yaml http://localhost:8080/api/packs
```

Abre **Devices → Add device**: *Shelly plug (Gen2+)* está en la lista.
Añade tu enchufe con su dirección; el primer sondeo llega en un minuto,
y la regla *Shelly plug overheating* está en **Alerts → Rules**.

Para publicar una versión nueva, eleva `version`, ejecuta `pack test`, e
instálalo de nuevo: la extracción se reemplaza, la regla que puedas
haber ajustado se conserva.

El pack completo, con su aviso de configuración, está en
[`packs/shelly-plug`](https://github.com/laupernoe/dumbmonit/tree/main/packs/shelly-plug).
Todo lo que el formato acepta está en la [referencia](format.md).

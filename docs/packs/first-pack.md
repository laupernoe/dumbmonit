# Write your first pack

In about ten minutes, this page builds the `shelly-plug` reference pack from
scratch: a Shelly smart plug (Plus, Pro or Gen3) read through its local API,
with power, energy and temperature, and an alert when it overheats.

You need Docker, and either a Shelly plug on your network or the captured
response below.

## 1. Look at what the device answers

Shelly's second-generation devices describe a relay with one call:

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

Save this answer as `shelly-plug/fixtures/status.json`: it is the fixture the
pack will be tested against, so you can work without the device at hand.

## 2. The header

Create `shelly-plug/pack.yaml`:

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

The `id` decides two names: the device type `pack.shelly-plug`, and the metric
prefix `dumbmonit_shelly_plug_`.

## 3. One source

A source is one request, always to the device's own address:

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

`{{option.switch_id}}` is replaced by the device's own setting, shown in the
device form as **Switch channel**. The file name of the fixture, `status.json`,
matches the source `id`.

## 4. The metrics

Each metric picks a value with a [JSONPath](https://www.rfc-editor.org/rfc/rfc9535)
expression. Names are written without the prefix:

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

`output` is a boolean: it becomes 1 or 0. The energy total only grows, so it is
a `counter`: charts and rules read it with `rate()`, and a restart of the plug
does not look like a drop.

## 5. Check it

```bash
docker run --rm -v "$PWD:/work" --user "$(id -u)" ghcr.io/noekan/dumbmonit:latest \
  pack test /work/shelly-plug --update
```

```text
ok: shelly-plug 1.0.0 (pack.shelly-plug, 6 metrics, 0 rules)
  wrote /work/shelly-plug/expected.prom
```

`expected.prom` shows exactly what the server would store from the fixture:

```text
# TYPE dumbmonit_shelly_plug_power_watts gauge
dumbmonit_shelly_plug_power_watts 8.9
# TYPE dumbmonit_shelly_plug_temperature_celsius gauge
dumbmonit_shelly_plug_temperature_celsius 41.3
…
```

Read it once. From now on, `pack test` without `--update` fails if a change to
the pack changes what it extracts, and prints the lines that differ.

## 6. An alert rule

Rules come with the pack and use its metrics by their full name. The threshold
can come from an option, so each plug can have its own:

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

Run `pack lint` to check the whole file: a rule citing a metric the pack does
not produce, an option that does not exist or a path that points elsewhere are
reported with where they are.

```bash
docker run --rm -v "$PWD:/work:ro" ghcr.io/noekan/dumbmonit:latest pack lint /work/shelly-plug
```

## 7. Install it

With an API token that has the `write` scope:

```bash
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @shelly-plug/pack.yaml http://localhost:8080/api/packs
```

Open **Devices → Add device**: *Shelly plug (Gen2+)* is in the list. Add your
plug with its address; the first poll comes within a minute, and the
*Shelly plug overheating* rule is in **Alerts → Rules**.

To publish a new version, raise `version`, run `pack test`, and install it
again: the extraction is replaced, the rule you may have tuned is kept.

The complete pack, with its setup notice, is in
[`packs/shelly-plug`](https://github.com/noekan/dumbmonit/tree/main/packs/shelly-plug).
Everything the format accepts is in the [reference](format.md).

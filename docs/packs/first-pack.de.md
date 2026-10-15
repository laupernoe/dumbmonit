# Ihr erstes Pack schreiben

In etwa zehn Minuten baut diese Seite das Referenzpaket
`shelly-plug` von Grund auf: eine Shelly-Smart-Steckdose (Plus, Pro
oder Gen3), über ihre lokale API gelesen, mit Leistung, Energie und
Temperatur, und einem Alarm bei Überhitzung.

Sie brauchen Docker, und entweder eine Shelly-Steckdose in Ihrem
Netzwerk oder die unten aufgezeichnete Antwort.

## 1. Betrachten, was das Gerät antwortet

Shellys Geräte der zweiten Generation beschreiben ein Relais mit
einem Aufruf:

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

Speichern Sie diese Antwort als `shelly-plug/fixtures/status.json`: Es
ist die Fixture, gegen die das Pack getestet wird, sodass Sie ohne
das Gerät zur Hand arbeiten können.

## 2. Der Header

Erstellen Sie `shelly-plug/pack.yaml`:

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

Die `id` entscheidet über zwei Namen: die Monitorart
`pack.shelly-plug`, und das Metrik-Präfix `dumbmonit_shelly_plug_`.

## 3. Eine Quelle

Eine Quelle ist ein Request, immer an die eigene Adresse des Geräts:

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

`{{option.switch_id}}` wird durch die eigene Einstellung des Monitors
ersetzt, im Monitordialog als **Switch channel** angezeigt. Der
Dateiname der Fixture, `status.json`, passt zur Quellen-`id`.

## 4. Die Metriken

Jede Metrik wählt einen Wert mit einem
[JSONPath](https://www.rfc-editor.org/rfc/rfc9535)-Ausdruck. Namen
werden ohne das Präfix geschrieben:

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

`output` ist ein Boolean: Er wird zu 1 oder 0. Das Energie-Total wächst
nur, daher ist es ein `counter`: Diagramme und Regeln lesen es mit
`rate()`, und ein Neustart der Steckdose sieht nicht wie ein Einbruch
aus.

## 5. Prüfen

```bash
docker run --rm -v "$PWD:/work" --user "$(id -u)" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /work/shelly-plug --update
```

```text
ok: shelly-plug 1.0.0 (pack.shelly-plug, 6 metrics, 0 rules)
  wrote /work/shelly-plug/expected.prom
```

`expected.prom` zeigt genau, was der Server aus der Fixture
speichern würde:

```text
# TYPE dumbmonit_shelly_plug_power_watts gauge
dumbmonit_shelly_plug_power_watts 8.9
# TYPE dumbmonit_shelly_plug_temperature_celsius gauge
dumbmonit_shelly_plug_temperature_celsius 41.3
…
```

Lesen Sie es einmal. Von nun an schlägt `pack test` ohne `--update`
fehl, wenn eine Änderung am Pack ändert, was es extrahiert, und gibt
die abweichenden Zeilen aus.

## 6. Eine Alarmregel

Regeln kommen mit dem Pack und nutzen seine Metriken bei ihrem
vollständigen Namen. Der Schwellenwert kann aus einer Option kommen,
sodass jede Steckdose ihren eigenen haben kann:

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

Führen Sie `pack lint` aus, um die gesamte Datei zu prüfen: Eine
Regel, die eine vom Pack nicht erzeugte Metrik zitiert, eine nicht
existierende Option oder ein anderswohin zeigender Pfad werden mit
ihrer Stelle gemeldet.

```bash
docker run --rm -v "$PWD:/work:ro" ghcr.io/laupernoe/dumbmonit:latest pack lint /work/shelly-plug
```

## 7. Installieren

Mit einem API-Token, das den Scope `write` hat:

```bash
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @shelly-plug/pack.yaml http://localhost:8080/api/packs
```

Öffnen Sie **Monitors → Add monitor**: *Shelly plug (Gen2+)* steht in
der Liste. Fügen Sie Ihre Steckdose mit ihrer Adresse hinzu; die
erste Abfrage kommt innerhalb einer Minute, und die Regel *Shelly
plug overheating* steht unter **Alerts → Rules**.

Um eine neue Version zu veröffentlichen, heben Sie `version` an,
führen Sie `pack test` aus, und installieren Sie erneut: Die
Extraktion wird ersetzt, die von Ihnen angepasste Regel bleibt
erhalten.

Das vollständige Pack, mit seinem Einrichtungshinweis, liegt in
[`packs/shelly-plug`](https://github.com/laupernoe/dumbmonit/tree/main/packs/shelly-plug).
Alles, was das Format akzeptiert, steht in der
[Referenz](format.md).

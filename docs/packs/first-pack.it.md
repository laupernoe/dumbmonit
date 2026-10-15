# Scrivi il tuo primo pacchetto

In circa dieci minuti, questa pagina costruisce da zero il pacchetto di
riferimento `shelly-plug`: una presa intelligente Shelly (Plus, Pro o
Gen3) letta tramite la sua API locale, con potenza, energia e
temperatura, e un avviso quando si sovrariscalda.

Hai bisogno di Docker, e di una presa Shelly sulla tua rete oppure della
risposta catturata sotto.

## 1. Guarda cosa risponde il dispositivo

I dispositivi di seconda generazione di Shelly descrivono un relè con
una chiamata:

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

Salva questa risposta come `shelly-plug/fixtures/status.json`: è la
fixture contro cui il pacchetto verrà testato, così puoi lavorare senza
avere il dispositivo a portata di mano.

## 2. L'intestazione

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

L'`id` decide due nomi: il tipo di monitor `pack.shelly-plug`, e il
prefisso di metrica `dumbmonit_shelly_plug_`.

## 3. Una fonte

Una fonte è una richiesta, sempre verso l'indirizzo proprio del
monitor:

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

`{{option.switch_id}}` viene sostituito dall'impostazione propria del
monitor, mostrata nel modulo del monitor come **Switch channel**.
Il nome del file della fixture, `status.json`, corrisponde all'`id`
della fonte.

## 4. Le metriche

Ogni metrica sceglie un valore con un'espressione
[JSONPath](https://www.rfc-editor.org/rfc/rfc9535). I nomi sono scritti
senza il prefisso:

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

`output` è un booleano: diventa 1 o 0. Il totale dell'energia cresce
solo, quindi è un `counter`: grafici e regole lo leggono con `rate()`, e
un riavvio della presa non appare come un calo.

## 5. Verificalo

```bash
docker run --rm -v "$PWD:/work" --user "$(id -u)" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /work/shelly-plug --update
```

```text
ok: shelly-plug 1.0.0 (pack.shelly-plug, 6 metrics, 0 rules)
  wrote /work/shelly-plug/expected.prom
```

`expected.prom` mostra esattamente ciò che il server memorizzerebbe dalla
fixture:

```text
# TYPE dumbmonit_shelly_plug_power_watts gauge
dumbmonit_shelly_plug_power_watts 8.9
# TYPE dumbmonit_shelly_plug_temperature_celsius gauge
dumbmonit_shelly_plug_temperature_celsius 41.3
…
```

Leggilo una volta. Da adesso, `pack test` senza `--update` fallisce se
una modifica al pacchetto cambia ciò che estrae, e stampa le righe che
differiscono.

## 6. Una regola di avviso

Le regole vengono con il pacchetto e usano le sue metriche con il loro
nome completo. La soglia può provenire da un'opzione, così ogni presa può
avere la propria:

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

Esegui `pack lint` per verificare l'intero file: una regola che cita una
metrica che il pacchetto non produce, un'opzione che non esiste o un
percorso che punta altrove vengono segnalati con dove si trovano.

```bash
docker run --rm -v "$PWD:/work:ro" ghcr.io/laupernoe/dumbmonit:latest pack lint /work/shelly-plug
```

## 7. Installalo

Con un token API che ha l'ambito `write`:

```bash
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @shelly-plug/pack.yaml http://localhost:8080/api/packs
```

Apri **Devices → Add device**: *Shelly plug (Gen2+)* è nell'elenco.
Aggiungi la tua presa con il suo indirizzo; il primo sondaggio arriva
entro un minuto, e la regola *Shelly plug overheating* è in
**Alerts → Rules**.

Per pubblicare una nuova versione, alza `version`, esegui `pack test`, e
installala di nuovo: l'estrazione viene sostituita, la regola che
potresti aver messo a punto viene conservata.

Il pacchetto completo, con la sua nota di configurazione, è in
[`packs/shelly-plug`](https://github.com/laupernoe/dumbmonit/tree/main/packs/shelly-plug).
Tutto ciò che il formato accetta è nel [riferimento](format.md).

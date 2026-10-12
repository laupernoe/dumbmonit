# Pack-Format-Referenz

Ein Pack ist eine YAML-Datei, `pack.yaml`, in Formatversion 1.
Unbekannte Schlüssel sind Fehler, nicht ignorierte Einstellungen: Ein
Tippfehler wie `treshold` wird gemeldet.

```text
my-pack/
  pack.yaml          das Pack selbst: die einzige Datei, die der Server braucht
  fixtures/          eine aufgezeichnete Antwort pro Quelle, nach ihrer id benannt
    status.json
  expected.prom      was die Extraktion aus den Fixtures erzeugen muss
```

## Header

| Schlüssel | Erforderlich | Bedeutung |
|---|---|---|
| `schema` | ja | Immer `1`. |
| `id` | ja | Kleinbuchstaben, Ziffern und einzelne Bindestriche, beginnend mit einem Buchstaben, bis zu 40 Zeichen: `shelly-plug`. Die Geräteart wird `pack.<id>`, das Metrik-Präfix `dumbmonit_<id mit Unterstrichen>_`. |
| `version` | ja | [Semantische Version](https://semver.org) des Pakets: `1.2.0`. |
| `requires` | nein | DumbMonit-Versionen, mit denen das Paket funktioniert, als Semver-Anforderung: `">=0.2"`. Ein Server außerhalb des Bereichs lehnt das Paket ab. |
| `label` | ja | In der Geräteartenliste angezeigter Name. |
| `summary` | nein | Ein Satz: was das Paket beobachtet. |
| `examples` | nein | Produkte, für die es gilt, als Hinweise angezeigt. |
| `address_hint` | nein | Platzhalter des Adressfelds: `192.168.1.50`. |
| `default_port` | nein | Genutzter Port, wenn die Adresse keinen angibt. |
| `scheme` | nein | `http` (Standard) oder `https`; das Gerät kann es überschreiben. |
| `setup` | nein | Der neben dem Gerätedialog gezeigte Hinweis: `title`, `steps` (eine Liste; was einem Zeilenumbruch in einem Schritt folgt, wird als kopierbarer Befehl gezeigt), `warning`, `doc_url`. |
| `credentials` | nein | Akzeptierte Credential-Arten, in Präferenzreihenfolge: `none`, `api_token`, `username_password`. Standard: `[none]`. Ein Eintrag kann `{kind, label, help}` sein, um das Formular umzuformulieren. |
| `options` | nein | Im Gerätedialog angezeigte Einstellungen (unten). |

## Optionen

Optionen werden am Gerät gespeichert und mit `{{option.<key>}}` in
Pfaden, Headern und Bodies, und von Regel-Schwellenwerten gelesen.

```yaml
options:
  - key: switch_id          # Kleinbuchstaben, Ziffern, Unterstriche
    label: Switch channel
    help: 0 for a plug; 0, 1, 2… for each channel of a relay.
    input: number           # text (Standard), number, boolean oder select
    default: 0
    required: false
    placeholder: ""
    choices: []             # die Werte eines select
```

Jedes abfragende Paket erhält zusätzlich fünf Verbindungsoptionen,
die es nicht neu deklarieren kann: `scheme`, `port`, `insecure_tls`,
`request_timeout_seconds` (1 bis 120, Standard 10) und
`allow_private_targets`.

## Quellen

Eine Quelle ist ein Request pro Abfrage, immer an die eigene Adresse
des Geräts gesendet.

```yaml
sources:
  - id: status              # Kleinbuchstaben, Ziffern, Unterstriche
    type: http              # http oder prometheus
    path: /rpc/Switch.GetStatus?id={{option.switch_id}}
    method: GET             # GET (Standard) oder POST
    format: json            # nur http: json (Standard) oder text
    headers:
      Accept: application/json
    body: '{"id": {{option.switch_id}}}'   # nur POST
    auth: auto              # auto (Standard), none, bearer oder basic
```

- `path` muss mit einem einzelnen `/` beginnen. Die Request-URL ist
  die Adresse des Geräts gefolgt vom Pfad; nach der Ersetzung muss sie
  dasselbe Schema, denselben Host und Port behalten, sonst wird der
  Request abgelehnt. Options-Werte werden in Pfaden percent-codiert,
  sodass ein Wert kein Segment hinzufügen oder den Host ändern kann.
- `auth: auto` sendet ein `api_token`-Credential als
  `Authorization: Bearer …` und ein `username_password`-Credential
  als Basic-Authentifizierung. `none` sendet implizit nichts, sodass
  ein Header das Credential stattdessen tragen kann:
  `X-Api-Key: "{{credential.token}}"`. `{{credential.token}}`,
  `{{credential.username}}` und `{{credential.password}}` werden
  **nur in Headern** akzeptiert: nie in einem Pfad oder einem Body,
  die in Logs landen.
- Weiterleitungen werden nur zu demselben Schema, Host und Port
  gefolgt, höchstens drei. `401` und `403` werden als
  Authentifizierungsfehler gemeldet, andere Nicht-2xx-Status als
  Protokollfehler.
- Eine Abfrage sendet höchstens 16 Requests und liest höchstens 4 MiB
  pro Antwort.

### Prometheus-Quellen

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

- `keep` listet die Familien, so wie sie importiert werden, mit allen
  ihren Labels. Ein Familienname deckt auch seine Histogramm- und
  Summary-Zeitreihen ab (`_bucket`, `_sum`, `_count`). Zähler,
  Histogramme und Summary-Sums bleiben Zähler; alles andere wird ein
  Gauge.
- `rename` gibt einer behaltenen Familie ihren DumbMonit-Namen. Ein
  Name, der nicht aus Kleinbuchstaben, Ziffern und Unterstrichen
  besteht, muss umbenannt werden.
- `drop` verwirft jeden Sample, dessen Label auf den regulären
  Ausdruck passt.
- Von DumbMonit selbst gesetzte Labels (`target`, `host`, `instance`,
  `job`, `tag_*`…) werden als `exported_<name>` behalten.

## Metriken

Jede Metrik hat einen Namen *ohne* Präfix: `power_watts` im Paket
`shelly-plug` wird zu `dumbmonit_shelly_plug_power_watts`. Namen sind
Kleinbuchstaben, Ziffern und Unterstriche, und innerhalb des Pakets
eindeutig.

```yaml
metrics:
  - name: power_watts
    help: Active power drawn by the load.
    source: status
    kind: gauge             # gauge (Standard) oder counter
    json: $.apower          # genau eines von json, regex oder prom
    scale: 1                # Multiplikator, z. B. 0,001 für ms → s
    map: {on: 1, off: 0}    # Textwert → Zahl
    labels: {}
```

| Extraktor | Quelle | Wert | `labels` |
|---|---|---|---|
| `json: <JSONPath>` | `http`, Format `json` | Jeder vom Pfad passende Knoten ([RFC 9535](https://www.rfc-editor.org/rfc/rfc9535) JSONPath, Wildcards und Filter eingeschlossen) ergibt eine Zeitreihe. | Label → JSONPath, ausgewertet auf dem Objekt, das den Wert *enthält*: `$.sensors[*].value` mit Labels `sensor: $.name` beschriftet jeden Wert mit seinem Geschwister `name`. |
| `regex: <Ausdruck>` | `http`, Format `text` | Jede Übereinstimmung ergibt eine Zeitreihe; der Wert ist die Gruppe namens `value`, sonst die erste Gruppe. | Label → Name einer Capture-Gruppe. |
| `prom: <Sample-Name>` | `prometheus` | Jeder Sample mit genau diesem Namen. | Label → Quell-Label. Leer: alle Quell-Labels werden behalten. |

Werte: Zahlen werden so genommen, wie sie sind; `true`/`false` werden
zu 1/0; Text wird in `map` nachgeschlagen, oder, ohne `map`, als Zahl
geparst. Ein fehlender, `null`-Wert, oder einer außerhalb von `map`,
erzeugt für diese Abfrage nichts.

## Discover

`discover` verwandelt jedes Element einer Liste in einen Satz von
Zeitreihen mit gemeinsamen Labels: eine Zeile pro Platte, Sensor oder
Warteschlange.

```yaml
discover:
  - name: disks
    source: list
    rows: $.disks[*]        # JSONPath: eine Zeile pro Treffer
    max_rows: 100           # Standard 100, höchstens 1.000
    labels:                 # JSONPath, auf jeder Zeile ausgewertet
      disk: $.name
      model: $.info.model
    metrics:
      - name: disk_used_bytes
        json: $.used        # JSONPath, auf jeder Zeile ausgewertet
      - name: disk_healthy
        json: $.state
        map: {ok: 1, degraded: 0}
```

Zeilen-Metriken akzeptieren `name`, `help`, `kind`, `json`, `scale`
und `map`. Zeilen über `max_rows` hinaus werden ignoriert.

## Regeln

Regeln werden mit dem Paket installiert, als gewöhnliche Alarmregeln
mit der uid `pack:<id>:<name>`. Sie gehören dann Ihnen: Ein Update des
Pakets fügt die noch fehlenden Regeln hinzu und überschreibt nie eine
von Ihnen geänderte oder gelöschte.

```yaml
rules:
  - name: overheating       # Kleinbuchstaben, Ziffern, Unterstriche
    title: Shelly plug overheating
    description: The internal temperature stayed above the limit for five minutes.
    expr: dumbmonit_shelly_plug_temperature_celsius
    op: ">"                 # >, >=, < oder <=
    threshold: "{{option.max_temperature}}"   # oder eine Zahl
    for: 5m                 # 30s, 5m, 1h, 1d
    severity: warning       # info, warning (Standard) oder critical
    unit: °C
```

- `expr` ist [MetricsQL](https://docs.victoriametrics.com/metricsql/)
  und muss mindestens eine Metrik des Pakets nutzen, bei ihrem
  vollständigen Namen. Jeder andere zitierte `dumbmonit_*`-Name muss
  ebenfalls vom Paket erzeugt werden (`dumbmonit_up` ausgenommen):
  Ein Paket kann nicht auf Zeitreihen einer anderen Art alarmieren.
- Ein `threshold` von `{{option.x}}` installiert den Standardwert der
  Option, und der eigene Wert jedes Geräts für diese Option wird
  dessen Schwellenwert für diese Regel (eine gerätespezifische
  Überschreibung, bei jedem Speichern des Geräts neu geschrieben).

## SNMP-Profile

Ein Paket kann SNMP-Profile mitbringen, genauso geschrieben wie die
eingebauten in
[`profiles/`](https://github.com/laupernoe/dumbmonit/tree/main/profiles).
Sie treten dem SNMP-Katalog bei und werden wie die anderen per
`sysObjectID` angewendet; sie erstellen keine eigene Geräteart.

```yaml
snmp_profiles:
  - id: acme-ups            # die Paket-id, oder beginnend mit "<Paket-id>-"
    name: ACME UPS
    match:
      sysobjectid: [1.3.6.1.4.1.99999]
    include: [system]
    metrics:
      - name: acme_ups_battery_percent   # muss mit dem Paket-Präfix beginnen
        oid: 1.3.6.1.4.1.99999.1.1.0
        kind: gauge
```

Der SNMP-Katalog wird beim Start gelesen: Starten Sie DumbMonit nach
dem Installieren oder Aktualisieren eines Pakets mit SNMP-Profilen
neu.

## Limits

| Limit | Wert |
|---|---|
| Paketgröße | 256 KiB |
| Quellen (Requests pro Abfrage, Weiterleitungen eingeschlossen) | 16 |
| Antwortgröße | 4 MiB |
| Zeitreihen pro Abfrage | 5.000 (der Rest wird verworfen und protokolliert) |
| Deklarierte Metriken | 256 |
| Regeln | 64 |
| Zeilen pro `discover` | 1.000 |
| Label-Wertlänge | 200 Bytes |

## Fixtures und erwartete Ausgabe

`dumbmonit pack test <dir>` liest `fixtures/<Quellen-id>.json`,
`.txt` oder `.prom`, führt dieselbe Extraktion wie der Server aus,
und vergleicht das Ergebnis mit `expected.prom`: das Prometheus-
Textformat, sortiert, ohne Zeitstempel und ohne die vom Server
hinzugefügten Gerätelabels (`target`, `host`, `tag_*`). `--update`
schreibt `expected.prom` aus den Fixtures; prüfen Sie den Diff, bevor
Sie ihn committen.

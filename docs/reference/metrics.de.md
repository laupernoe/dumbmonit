# Metriken {#metrics}

DumbMonit speichert jede Messung in VictoriaMetrics und liest sie mit MetricsQL über den
[Metrik-Proxy](api.md#metrics) des Servers zurück. Diese Seite beschreibt Benennung, Labels und
Aufbewahrung. Die Metriklisten pro Typ stehen auf den [Geräteseiten](../devices/index.md).

## Benennung {#naming}

Jede Metrik hat das Präfix `dumbmonit_`, sodass DumbMonit sich ohne Kollisionen eine
VictoriaMetrics-Instanz mit anderen Werkzeugen teilen kann. Namen folgen
`dumbmonit_<what>_<unit>`:

| Muster | Beispiele |
|---|---|
| `…_bytes`, `…_bytes_total`, `…_bytes_used` | `dumbmonit_memory_bytes_total`, `dumbmonit_filesystem_used_bytes` |
| `…_percent` | `dumbmonit_cpu_usage_percent`, `dumbmonit_pbs_datastore_used_percent` |
| `…_seconds`, `…_age_seconds` | `dumbmonit_probe_duration_seconds`, `dumbmonit_proxmox_backup_last_age_seconds` |
| `…_celsius`, `…_volts`, `…_hertz`, `…_watts` | USV- und Synology-Messwerte |
| `…_info` | Präsenzserien (Wert 1), deren Information in Labels steht: `dumbmonit_system_info`, `dumbmonit_pbs_version_info` |
| `…_status`, `…_up`, `…_running` | Aufzählungen oder Booleans: `dumbmonit_if_oper_status`, `dumbmonit_proxmox_guest_running` |

Quellspezifische Familien tragen ihre Quelle als zweites Präfix: `dumbmonit_proxmox_*`,
`dumbmonit_pbs_*`, `dumbmonit_synology_*`, `dumbmonit_probe_*` (Dienste). SNMP-Profile und der
Agent verwenden schlichte Namen (`dumbmonit_if_octets_in`, `dumbmonit_cpu_usage_percent`).

Zähler (Oktette und Pakete von Schnittstellen, Plattenbytes, gedruckte Seiten) werden **roh**
gespeichert, wie das Gerät sie meldet: Wenden Sie in Ihren eigenen Abfragen `rate()` an, um einen
Durchsatz zu erhalten. Die Geräteseite zeichnet jede Serie `dumbmonit_*` des Geräts wie
gespeichert, ein Zähler erscheint also als steigende Linie. Ein Neustart zeigt sich als
Zähler-Reset, nie als falsche Spitze.

## `dumbmonit_up` {#dumbmonit_up}

Nach jeder erfolgreichen Prüfung schreibt der Server `dumbmonit_up = 1` für das Gerät. Bei einer
fehlgeschlagenen Prüfung wird nichts geschrieben: Die Serie endet schlicht, und die Regel „Gerät
nicht erreichbar“ misst das Alter des letzten Messwerts
(`time() - tlast_over_time(dumbmonit_up[7d])`) und behält nur die frischeste Serie jedes Geräts
(`bottomk by (target) (1, …)`): Eine Serie, die nach einer Tag- oder Label-Änderung nicht mehr
geschrieben wird, bleibt im 7-Tage-Fenster und darf nicht zählen.

Bei Diensten (`http`, `tcp`, `dns`, `ping`, `tls`) bedeutet `dumbmonit_up = 1` „der Monitor lief“,
und `dumbmonit_probe_success` sagt, ob der Dienst in Ordnung ist. Die beiden Signale werden
absichtlich getrennt gehalten: Ein `probe_success` von 0 sagt, dass der Dienst ausgefallen ist,
eine Unterbrechung von `up` sagt, dass die Überwachung ausgefallen ist.

## `dumbmonit_security_score` {#dumbmonit_security_score}

Alle 15 Minuten berechnet der Server den [Sicherheitswert](../using/security-score.md) jedes
Geräts, dessen Art Sicherheitsprüfungen hat, und schreibt `dumbmonit_security_score` (0–100,
fehlt, solange nichts bewertet werden kann) und `dumbmonit_security_checks_failed`, beide mit dem
Label `target`.

## Labels {#labels}

| Label | Gesetzt von | Bedeutung |
|---|---|---|
| `target` | Server | Die Geräte-ID, wie in `/api/targets/{id}`. Jede Regel und jedes Diagramm wählt darüber aus. |
| `host` | Server | Der Gerätename. |
| `tag_<key>` | Server | Die freien Tags des Geräts. Typoptionen (`insecure_tls`, `port` …), Tags, deren Schlüssel wie ein Geheimnis aussieht (pro Wort geprüft: `key`, `pass`, `cert`, `private`, `pem` …), und Werte über 256 Zeichen werden nicht kopiert. Legen Sie nie ein Geheimnis in einem Tag ab. |
| `probe` | Dienste | `http`, `tcp`, `dns`, `ping` oder `tls`. |
| Labels pro Serie | Collector | `ifname`, `mountpoint`, `device`, `fstype`, `core`, `service`, `container`, `node`, `vmid`, `name`, `storage`, `datastore`, `group`, `volume`, `disk`, `pool`, `repo`, `changes`, `reason`, `version`, `issuer` … wie auf jeder Geräteseite aufgeführt. |

Identitätslabels haben immer Vorrang vor Collector-Labels, und bei Agenten werden sie beim
Empfang gesetzt: Ein Rechner kann nicht in die Serien eines anderen Rechners schreiben.

!!! note "Präfix vor September 2026"
    Von EzyMonit geschriebene Serien heißen `ezymonit_…`. Das Präfix änderte sich mit dem
    Produktnamen, und nichts liest die alten Serien: Sie bleiben in VictoriaMetrics, bis die
    Aufbewahrung sie entfernt, Diagramme und Regeln beginnen ab dem Upgrade neu, und
    benutzerdefinierte Regeln, die noch `ezymonit_…` nennen, müssen bearbeitet werden.

## Aufbewahrung und Speicherung {#retention-and-storage}

Das eingebettete VictoriaMetrics bewahrt 12 Monate Daten auf (`DUMBMONIT_VM_RETENTION`), unter
`/data/vm`, im Volume `dumbmonit-data`. Der Server bündelt seine Schreibvorgänge und schreibt sie
alle 5 Sekunden (`DUMBMONIT_FLUSH_INTERVAL_SECS`). Das Löschen eines Geräts löscht seine Serien
nicht; sie altern aus.

Der Alarmverlauf wird 90 Tage aufbewahrt, die Baselines von Serien, die 60 Tage verschwunden
waren, ebenso, beides in SQLite (siehe die [Konfigurationsreferenz](configuration.md)).

## Abfragen {#querying}

- Über den Server: `GET /api/metrics/query` und `GET /api/metrics/query_range`, mit dem
  Sitzungscookie. Bereichsergebnisse sind auf 2 000 Punkte pro Serie begrenzt.
- Aus einem bestehenden Prometheus oder Grafana: `GET /federate`, `GET /metrics` und die
  Datenquelle `/prometheus`, mit einem API-Token — [unten](#scraping-dumbmonit).
- Direkt, in der Entwicklung: Das Overlay `docker-compose.dev.yml` veröffentlicht das
  eingebettete VictoriaMetrics auf `http://localhost:8428`, mit eigener Oberfläche unter
  `http://localhost:8428/vmui/`. In Produktion lauscht es nur auf dem Loopback des Containers;
  setzen Sie `DUMBMONIT_VM_LISTEN=0.0.0.0:8428` und veröffentlichen Sie den Port selbst, wenn Sie
  Grafana oder ein anderes Werkzeug auf denselben Daten wollen.

```
# Interface throughput in bits per second
rate(dumbmonit_if_octets_in{target="4"}[5m]) * 8

# Availability of every service over 30 days
avg by (host) (avg_over_time(dumbmonit_probe_success[30d]))

# Every device that has not reported for 5 minutes
bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d])) > 300
```

## DumbMonit abfragen (Scraping) {#scraping-dumbmonit}

Wenn Sie bereits ein Prometheus oder Grafana betreiben, brauchen Sie keinen zweiten Stack. Drei
Routen liegen außerhalb von `/api`, dort, wo ein Scraper sie sucht:

| Route | Was sie liefert |
|---|---|
| `GET /metrics` | Den Zustand der DumbMonit-Instanz selbst: Scheduler, Prüfungen, Schreibvorgänge, Alarmierung, Benachrichtigungen, Agenten, Datenbank. Einige Dutzend Serien, immer dieselbe Anzahl — nie eine pro Gerät. |
| `GET /federate` | Die Messungen, per `match[]` ausgewählt, genau wie Prometheus ein anderes Prometheus föderiert: der jüngste Punkt jeder ausgewählten Serie. |
| `GET /prometheus/api/v1/…` | Die Lesehälfte der Prometheus-API, an den Speicher weitergereicht: das, was eine Grafana-Datenquelle aufruft. |

Alle drei nehmen `Authorization: Bearer dmt_…` mit dem Scope **`read`** — ein unter Einstellungen
→ *API & Assistenten* erstelltes Token, dieselbe Art, die der [MCP-Server](../using/assistant.md)
verwendet. Es ist kein Cookie im Spiel, daher ist kein Header `X-Requested-With` nötig; ein
Sitzungscookie wird ebenfalls akzeptiert, sodass Sie jede davon aus einem angemeldeten Browser
öffnen können. Ein Token ist auf 120 Aufrufe pro Minute begrenzt: reichlich für einen Scrape alle
15 Sekunden und genug für ein Grafana-Dashboard mit einigen Dutzend Panels — geben Sie einem
stark genutzten Dashboard ein eigenes Token, statt es mit einem Scraper zu teilen.

### Prometheus {#prometheus}

```yaml
# prometheus.yml — DumbMonit's own health, then the measurements it collects.
scrape_configs:
  - job_name: dumbmonit
    metrics_path: /metrics
    scheme: http
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    static_configs:
      - targets: ['dumbmonit.lan:8080']

  - job_name: dumbmonit-federate
    metrics_path: /federate
    scheme: http
    honor_labels: true
    scrape_interval: 60s
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    params:
      'match[]':
        - '{__name__=~"dumbmonit_.*"}'
    static_configs:
      - targets: ['dumbmonit.lan:8080']
```

`honor_labels: true` behält die von DumbMonit gesetzten Labels `target` und `host`, statt sie mit
denen des Scrape-Jobs zu überschreiben. Legen Sie das Token in eine Datei und verwenden Sie
`authorization: {type: Bearer, credentials_file: /etc/prometheus/dumbmonit.token}`, wenn Sie es
nicht in der Konfiguration haben möchten.

Ohne `match[]` wählt `/federate` `{__name__=~"dumbmonit_.*"}` — jede von DumbMonit geschriebene
Serie und nichts, was einem anderen Werkzeug im selben VictoriaMetrics gehört. Schränken Sie es
ein, um eine Föderation klein zu halten: `{__name__=~"dumbmonit_.*",target="4"}` für ein Gerät,
`{__name__="dumbmonit_up"}` nur für die Verfügbarkeit. Höchstens 10 Selektoren pro Anfrage, und
eine Antwort über 8 MiB wird mit einer Meldung abgelehnt, die Sie auffordert, einzuschränken —
teilen Sie eine große Flotte auf mehrere Jobs mit engeren Selektoren auf, statt auf einen, der
alles verlangt. `max_lookback=15m` weitet das Fenster aus, in dem der letzte Punkt gesucht wird,
für Geräte, die seltener als alle fünf Minuten geprüft werden.

```bash
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/metrics
curl -H 'Authorization: Bearer dmt_…' -G http://localhost:8080/federate \
  --data-urlencode 'match[]={__name__=~"dumbmonit_.*",target="4"}'
```

### Grafana {#grafana}

Richten Sie Grafana auf **DumbMonit**, nicht auf das VictoriaMetrics, das es betreibt: Der
Kindprozess lauscht auf dem Loopback des Containers, hat keine eigene Authentifizierung und
stellt die Routen, die Serien *löschen*, ebenso bereit wie die lesenden. DumbMonit reicht die
Lesehälfte der Prometheus-API unter `/prometheus` weiter, hinter demselben Token, sodass Grafana
mit einer normalen Prometheus-Datenquelle spricht:

1. **Connections → Data sources → Add → Prometheus**.
2. **URL**: `http://dumbmonit.lan:8080/prometheus` (Grafana hängt `/api/v1/query` selbst an).
3. **HTTP headers**: Fügen Sie einen hinzu, `Authorization` = `Bearer dmt_…`. Grafana speichert
   ihn verschlüsselt und zeigt ihn nie wieder an.
4. **Save & test** — Grafana meldet, dass die Datenquelle funktioniert, und jede Serie
   `dumbmonit_*` steht dann in PromQL zur Verfügung, mit Autovervollständigung bei Metrik- und
   Labelnamen.

Nur die lesenden Routen werden weitergereicht — `query`, `query_range`, `series`, `labels`,
Labelwerte, `metadata` und die Version, die Grafana beim Testen der Verbindung abfragt. Alles
andere, allen voran `admin/tsdb/delete_series`, antwortet mit `404`: Dieser Einstiegspunkt kann
nichts ändern oder löschen.

Betreiben Sie bereits ein Prometheus, funktioniert der andere Weg genauso gut: Prometheus
föderiert DumbMonit mit dem obigen Job, Grafana liest Prometheus, und die Abfragen sind
identisch.

### Instanzmetriken {#instance-metrics}

Alles, was `GET /metrics` bereitstellt. Werte werden gelesen, wenn Sie scrapen; die Zähler
`_total` sind monoton und werden nur beim Neustart des Servers zurückgesetzt
(`dumbmonit_uptime_seconds` verrät, wann das geschah).

| Metrik | Typ | Bedeutung |
|---|---|---|
| `dumbmonit_build_info{version}` | Gauge | Immer 1. Das Label `version` ist die laufende Serverversion, dieselbe, die `/api/health` meldet. |
| `dumbmonit_uptime_seconds` | Gauge | Sekunden seit dem Start des Serverprozesses. |
| `dumbmonit_scheduler_cycles_total` | Zähler | Ausgeführte Scheduler-Zyklen. Die Schleife tickt einmal pro Sekunde, das dient also zugleich als Lebenszeichen. |
| `dumbmonit_scheduler_cycle_seconds` | Gauge | Dauer des letzten Zyklus. Normalerweise unter einer Millisekunde: Der Zyklus verteilt nur, er wartet nicht auf Prüfungen. |
| `dumbmonit_scheduler_backlog` | Gauge | Geräte, die am Ende des letzten Zyklus noch auf eine Prüfung warteten. Null bei einer gesunden Instanz; ein dauerhafter Wert ungleich null bedeutet, dass `DUMBMONIT_MAX_CONCURRENT_PROBES` für die Anzahl der Geräte und ihre Intervalle zu niedrig ist. |
| `dumbmonit_probes_total{kind}` | Zähler | Ausgeführte Prüfungen, nach Geräteart (`snmp`, `proxmox`, `http` …). Schließt die ein, die ein Relay-Agent im Auftrag des Servers ausgeführt hat. |
| `dumbmonit_probes_failed_total{kind}` | Zähler | Fehlgeschlagene Prüfungen, nach Art — ein nicht erreichbares Gerät oder ein Konfigurationsfehler. `rate(dumbmonit_probes_failed_total[15m]) / rate(dumbmonit_probes_total[15m])` ist die Fehlerquote. |
| `dumbmonit_samples_written_total` | Zähler | Von VictoriaMetrics angenommene Messwerte. |
| `dumbmonit_sample_writes_failed_total` | Zähler | Batches, die VictoriaMetrics abgelehnt oder nicht beantwortet hat. Der Batch bleibt gepuffert und wird erneut versucht; ein steigender Wert zusammen mit einem steigenden `dumbmonit_samples_pending` bedeutet, dass der Speicher ausgefallen oder voll ist. |
| `dumbmonit_samples_pending` | Gauge | Nach dem letzten Flush noch im Schreibpuffer befindliche Messwerte. Normalerweise 0. |
| `dumbmonit_samples_dropped_total` | Zähler | Messwerte, die verworfen wurden, noch bevor sie den Schreibpuffer erreichten, weil er voll oder geschlossen war. Nie erneut versucht — ein steigender Wert während eines Ausfalls bedeutet Datenverlust, nicht nur Verzögerung. |
| `dumbmonit_alerting_cycles_total` | Zähler | Ausgeführte Alarmierungszyklen (einer alle `DUMBMONIT_ALERT_INTERVAL_SECS`, standardmäßig 30 s). |
| `dumbmonit_alerting_cycle_seconds` | Gauge | Dauer des letzten Alarmierungszyklus, Abfragezeit eingeschlossen. Nähert sie sich dem Intervall, fragen die Regeln mehr ab, als der Speicher liefern kann. |
| `dumbmonit_alerting_rules_evaluated` | Gauge | Im letzten Zyklus ausgewertete Regeln. |
| `dumbmonit_alerting_rules_failed` | Gauge | Regeln, deren Abfrage im letzten Zyklus fehlschlug. Ihre Alarme sind eingefroren, nicht aufgelöst. Alles außer 0 verdient einen Blick in die Logs. |
| `dumbmonit_alerts{phase}` | Gauge | Von der Engine verfolgte Alarme nach Phase: `ok`, `pending` (Bedingung wahr, `for` nicht abgelaufen), `firing`, `resolved`. |
| `dumbmonit_alerts_suppressed` | Gauge | Alarme, die verborgen sind, weil ein Elterngerät ausgefallen ist (die effektive Phase `suppressed`). |
| `dumbmonit_alerts_silenced` | Gauge | Von einem Wartungsfenster abgedeckte Alarme. |
| `dumbmonit_alerts_learning` | Gauge | Baseline-Alarme, die noch lernen und daher stumm sind. |
| `dumbmonit_notifications_total{kind}` | Zähler | Gesendete Benachrichtigungen, nach Kanalart (`email`, `discord`, `ntfy` …). |
| `dumbmonit_notifications_failed_total{kind}` | Zähler | Vom Kanal abgelehnte Benachrichtigungen, nach Art. Sie werden erneut eingereiht und wiederholt; ein stetig steigender Wert ist ein defekter Webhook oder SMTP-Konto. |
| `dumbmonit_agents` | Gauge | Rechner, die einen Agenten registriert haben. |
| `dumbmonit_agents_stale` | Gauge | Agenten, die seit fünf Minuten nichts gesendet haben. |
| `dumbmonit_database_up` | Gauge | 1, wenn SQLite antwortet — dieselbe Prüfung wie `/api/health`. |
| `dumbmonit_database_bytes` | Gauge | Größe von `dumbmonit.db` und seinem Write-Ahead-Log. Schließt `/data/vm` nicht ein. |
| `dumbmonit_victoriametrics_up` | Gauge | 1, wenn VictoriaMetrics antwortet — dieselbe Prüfung wie `/api/health`. 0, während der eingebettete Kindprozess neu startet. |
| `dumbmonit_victoriametrics_embedded` | Gauge | 1, wenn dieser Server VictoriaMetrics selbst betreibt, 0, wenn `DUMBMONIT_VM_URL` auf ein externes zeigt. |

Keine Metrik auf dieser Seite trägt ein Label pro Gerät: Das Dokument ist für ein Gerät und für
tausend gleich groß, und kein Gerätename gelangt an jemanden, der es scrapen kann.

Zwei Alarme, die Sie auf Ihrer Seite ergänzen sollten, sobald DumbMonit gescrapt wird:

```
# The monitoring stopped monitoring
up{job="dumbmonit"} == 0

# Measurements are piling up instead of being stored
dumbmonit_samples_pending > 0 and increase(dumbmonit_sample_writes_failed_total[10m]) > 0
```

### Die Routen ohne Token öffnen {#opening-the-routes-without-a-token}

`DUMBMONIT_METRICS_PUBLIC=true` liefert `/metrics`, `/federate` und `/prometheus/api/v1/…` an
jeden aus, der den Port erreicht, für Leute, die den Port selbst filtern — **jeder in diesem
Netzwerk kann dann jede Messung jedes Geräts und den Zustand der Instanz lesen, setzen Sie es also
nur, wenn bereits etwas anderes einschränkt, wer den Port erreicht.**

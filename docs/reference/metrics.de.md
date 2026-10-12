# Metriken

DumbMonit speichert jede Messung in VictoriaMetrics und liest sie mit
MetricsQL zurück, über den [Metrik-Proxy](api.md#metrics) des
Servers. Diese Seite beschreibt die Benennung, die Labels und die
Aufbewahrung. Pro-Art-Metriklisten stehen auf den
[Geräteseiten](../devices/index.md).

## Benennung

Jede Metrik hat das Präfix `dumbmonit_`, sodass DumbMonit eine
VictoriaMetrics-Instanz ohne Kollisionen mit anderen Tools teilen
kann. Namen folgen `dumbmonit_<was>_<einheit>`:

| Muster | Beispiele |
|---|---|
| `…_bytes`, `…_bytes_total`, `…_bytes_used` | `dumbmonit_memory_bytes_total`, `dumbmonit_filesystem_used_bytes` |
| `…_percent` | `dumbmonit_cpu_usage_percent`, `dumbmonit_pbs_datastore_used_percent` |
| `…_seconds`, `…_age_seconds` | `dumbmonit_probe_duration_seconds`, `dumbmonit_proxmox_backup_last_age_seconds` |
| `…_celsius`, `…_volts`, `…_hertz`, `…_watts` | USV- und Synology-Messwerte |
| `…_info` | Vorhandensein-Zeitreihen (Wert 1), deren Information in Labels liegt: `dumbmonit_system_info`, `dumbmonit_pbs_version_info` |
| `…_status`, `…_up`, `…_running` | Enumerationen oder Booleans: `dumbmonit_if_oper_status`, `dumbmonit_proxmox_guest_running` |

Quellenspezifische Familien tragen ihre Quelle als zweites Präfix:
`dumbmonit_proxmox_*`, `dumbmonit_pbs_*`, `dumbmonit_synology_*`,
`dumbmonit_probe_*` (Services). SNMP-Profile und der Agent nutzen
einfache Namen (`dumbmonit_if_octets_in`, `dumbmonit_cpu_usage_percent`).

Zähler (Schnittstellen-Octets und -Pakete, Platten-Bytes, gedruckte
Seiten) werden **roh** gespeichert, wie das Gerät sie meldet: Wenden
Sie `rate()` in Ihren eigenen Abfragen an, um einen Durchsatz zu
erhalten. Die Geräteseite plottet jede `dumbmonit_*`-Zeitreihe des
Geräts wie gespeichert, daher erscheint ein Zähler als steigende
Linie. Ein Reboot zeigt sich als Zähler-Reset, nie als gefälschter
Ausschlag.

## `dumbmonit_up`

Nach jeder erfolgreichen Abfrage schreibt der Server
`dumbmonit_up = 1` für das Gerät. Nichts wird geschrieben, wenn eine
Abfrage fehlschlägt: Die Zeitreihe stoppt einfach, und die Regel
"Device unreachable" misst das Alter des letzten Samples
(`time() - tlast_over_time(dumbmonit_up[7d])`).

Für Services (`http`, `tcp`, `dns`, `ping`, `tls`) bedeutet
`dumbmonit_up = 1` "der Monitor lief", und `dumbmonit_probe_success`
sagt, ob der Dienst in Ordnung ist. Die zwei Signale werden absichtlich
getrennt gehalten: Ein `probe_success` von 0 sagt, der Dienst ist
down, eine Unterbrechung von `up` sagt, die Überwachung ist down.

## `dumbmonit_security_score`

Alle 15 Minuten berechnet der Server den
[Security Score](../using/security-score.md) jedes Geräts, dessen Art
Sicherheitsprüfungen hat, und schreibt `dumbmonit_security_score`
(0–100, fehlt, solange nichts bewertet werden kann) und
`dumbmonit_security_checks_failed`, beide mit `target` beschriftet.

## Labels

| Label | Gesetzt von | Bedeutung |
|---|---|---|
| `target` | Server | Die Geräte-ID, wie in `/api/targets/{id}`. Jede Regel und jedes Diagramm wählt darüber aus. |
| `host` | Server | Der Gerätename. |
| `tag_<key>` | Server | Jedes Tag des Geräts, einschließlich Art-Optionen (`tag_insecure_tls`, `tag_port`…). Legen Sie nie ein Secret in ein Tag. |
| `probe` | Services | `http`, `tcp`, `dns`, `ping` oder `tls`. |
| Pro-Zeitreihe-Labels | Collector | `ifname`, `mountpoint`, `device`, `fstype`, `core`, `service`, `container`, `node`, `vmid`, `name`, `storage`, `datastore`, `group`, `volume`, `disk`, `pool`, `repo`, `changes`, `reason`, `version`, `issuer`… wie auf jeder Geräteseite aufgeführt. |

Identitätslabels gewinnen immer über Collector-Labels, und für Agents
werden sie beim Empfang gesetzt: Eine Maschine kann nicht in die
Zeitreihe einer anderen Maschine schreiben.

!!! note "Präfix vor September 2026"
    Von EzyMonit geschriebene Zeitreihen heißen `ezymonit_…`. Das
    Präfix änderte sich mit dem Produktnamen, und nichts liest die
    alten Zeitreihen: Sie bleiben in VictoriaMetrics, bis die
    Aufbewahrung sie entfernt, Diagramme und Regeln starten ab dem
    Upgrade neu, und eigene Regeln, die noch `ezymonit_…` benennen,
    müssen bearbeitet werden.

## Aufbewahrung und Speicherung

Das eingebettete VictoriaMetrics behält 12 Monate Daten
(`DUMBMONIT_VM_RETENTION`) unter `/data/vm`, im Volume
`dumbmonit-data`. Der Server bündelt seine Schreibvorgänge und
spült sie alle 5 Sekunden (`DUMBMONIT_FLUSH_INTERVAL_SECS`). Das
Löschen eines Geräts löscht seine Zeitreihen nicht; sie altern aus.

Die Alarmhistorie wird 90 Tage behalten und die Baselines
verschwundener Zeitreihen 60 Tage, beide in SQLite (siehe die
[Konfigurationsreferenz](configuration.md)).

## Abfragen

- Über den Server: `GET /api/metrics/query` und
  `GET /api/metrics/query_range`, mit dem Session-Cookie.
  Range-Ergebnisse sind auf 2.000 Punkte pro Zeitreihe begrenzt.
- Von einem bestehenden Prometheus oder Grafana:
  `GET /federate`, `GET /metrics` und die `/prometheus`-Datenquelle,
  mit einem API-Token — [unten](#scraping-dumbmonit).
- Direkt, in der Entwicklung: Das Overlay `docker-compose.dev.yml`
  veröffentlicht das eingebettete VictoriaMetrics auf
  `http://localhost:8428`, mit seiner eigenen UI auf
  `http://localhost:8428/vmui/`. In der Produktion hört es nur auf
  dem Loopback des Containers; setzen Sie `DUMBMONIT_VM_LISTEN=0.0.0.0:8428`
  und veröffentlichen Sie den Port selbst, wenn Sie Grafana oder ein
  anderes Tool auf denselben Daten wollen.

```
# Interface throughput in bits per second
rate(dumbmonit_if_octets_in{target="4"}[5m]) * 8

# Availability of every service over 30 days
avg by (host) (avg_over_time(dumbmonit_probe_success[30d]))

# Every device that has not reported for 5 minutes
time() - tlast_over_time(dumbmonit_up[7d]) > 300
```

## DumbMonit abfragen {#scraping-dumbmonit}

Wenn Sie bereits ein Prometheus oder ein Grafana betreiben, brauchen
Sie keinen zweiten Stack. Drei Routen sitzen außerhalb von `/api`,
wo ein Scraper nach ihnen sucht:

| Route | Was sie zurückgibt |
|---|---|
| `GET /metrics` | Die Gesundheit der DumbMonit-Instanz selbst: Scheduler, Abfragen, Schreibvorgänge, Alarmierung, Benachrichtigungen, Agents, Datenbank. Ein paar Dutzend Zeitreihen, immer dieselbe Anzahl — nie eine pro Gerät. |
| `GET /federate` | Die von `match[]` ausgewählten Messungen, genau wie Prometheus ein anderes Prometheus föderiert: der neueste Punkt jeder ausgewählten Zeitreihe. |
| `GET /prometheus/api/v1/…` | Die Lesehälfte der Prometheus-API, zum Store weitergeleitet: was eine Grafana-Datenquelle aufruft. |

Alle drei nehmen `Authorization: Bearer dmt_…` mit dem Scope
**`read`** — einem unter Settings → *API & assistants* erstellten
Token, derselben Art, die der [MCP-Server](../using/assistant.md)
nutzt. Kein Cookie ist involviert, daher wird kein
`X-Requested-With`-Header benötigt; ein Session-Cookie wird auch
akzeptiert, sodass Sie jede davon aus einem eingeloggten Browser
öffnen können. Ein Token ist auf 120 Aufrufe pro Minute begrenzt:
Reichlich für ein Scraping alle 15 Sekunden, und genug für ein
Grafana-Dashboard mit ein paar Dutzend Panels — geben Sie einem
viel genutzten Dashboard sein eigenes Token, statt eines mit einem
Scraper zu teilen.

### Prometheus

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

`honor_labels: true` behält die von DumbMonit gesetzten Labels
`target` und `host`, statt sie mit denen des Scrape-Jobs zu
überschreiben. Legen Sie das Token in eine Datei und nutzen Sie
`authorization: {type: Bearer, credentials_file: /etc/prometheus/dumbmonit.token}`,
wenn Sie es nicht in der Konfiguration haben wollen.

Ohne `match[]` wählt `/federate` `{__name__=~"dumbmonit_.*"}` —
jede von DumbMonit geschriebene Zeitreihe, und nichts, was einem
anderen Tool gehört, das sich dasselbe VictoriaMetrics teilt.
Begrenzen Sie es, um eine Föderation klein zu halten:
`{__name__=~"dumbmonit_.*",target="4"}` für ein Gerät,
`{__name__="dumbmonit_up"}` nur für Verfügbarkeit. Höchstens 10
Selektoren pro Request, und eine Antwort über 8 MiB wird mit einer
Meldung abgelehnt, die Sie bittet, einzugrenzen — teilen Sie eine
große Flotte in mehrere Jobs mit engeren Selektoren, statt einen,
der nach allem fragt.
`max_lookback=15m` erweitert das Fenster, in dem der letzte Punkt
gesucht wird, für seltener als alle fünf Minuten abgefragte Geräte.

```bash
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/metrics
curl -H 'Authorization: Bearer dmt_…' -G http://localhost:8080/federate \
  --data-urlencode 'match[]={__name__=~"dumbmonit_.*",target="4"}'
```

### Grafana

Richten Sie Grafana auf **DumbMonit**, nicht auf das von ihm
betriebene VictoriaMetrics: Der Kindprozess hört auf dem Loopback des
Containers, hat keine eigene Authentifizierung, und exponiert die
Routen, die Zeitreihen *löschen*, genauso bereitwillig wie die, die
sie lesen. DumbMonit leitet die Lesehälfte der Prometheus-API unter
`/prometheus` weiter, hinter demselben Token, sodass Grafana mit
einer normalen Prometheus-Datenquelle spricht:

1. **Connections → Data sources → Add → Prometheus**.
2. **URL**: `http://dumbmonit.lan:8080/prometheus` (Grafana hängt
   `/api/v1/query` selbst an).
3. **HTTP headers**: eines hinzufügen, `Authorization` =
   `Bearer dmt_…`. Grafana speichert es verschlüsselt und zeigt es
   nie wieder.
4. **Save & test** — Grafana meldet, die Datenquelle funktioniere,
   und jede `dumbmonit_*`-Zeitreihe ist dann in PromQL verfügbar, mit
   Autovervollständigung für Metrik- und Label-Namen.

Nur die Leseroutinen werden weitergeleitet — `query`, `query_range`,
`series`, `labels`, Label-Werte, `metadata` und die beim Testen der
Verbindung von Grafana abgefragte Version. Alles andere, vor allem
`admin/tsdb/delete_series`, antwortet mit `404`: Dieser Einstiegspunkt
kann nichts ändern oder löschen.

Wenn Sie bereits ein Prometheus betreiben, funktioniert der andere Weg
genauso gut: Prometheus föderiert DumbMonit mit dem obigen Job,
Grafana liest Prometheus, und die Abfragen sind identisch.

### Instanz-Metriken

Alles, was `GET /metrics` exponiert. Werte werden beim Scrapen
gelesen; die `_total`-Zähler sind monoton und setzen sich nur zurück,
wenn der Server neu startet (`dumbmonit_uptime_seconds` sagt Ihnen,
wann das passiert ist).

| Metrik | Typ | Bedeutung |
|---|---|---|
| `dumbmonit_build_info{version}` | Gauge | Immer 1. Das Label `version` ist die laufende Serverversion, dieselbe, die `/api/health` meldet. |
| `dumbmonit_uptime_seconds` | Gauge | Sekunden seit Start des Serverprozesses. |
| `dumbmonit_scheduler_cycles_total` | Zähler | Gelaufene Scheduler-Zyklen. Die Schleife tickt einmal pro Sekunde, daher dient dies auch als Lebendigkeitssignal. |
| `dumbmonit_scheduler_cycle_seconds` | Gauge | Dauer des letzten Zyklus. Normalerweise unter einer Millisekunde: Der Zyklus verteilt nur, er wartet nicht auf Abfragen. |
| `dumbmonit_scheduler_backlog` | Gauge | Geräte, die noch eine Abfrage fällig hatten, als der letzte Zyklus endete. Null auf einer gesunden Instanz; ein dauerhaft von null abweichender Wert bedeutet, dass `DUMBMONIT_MAX_CONCURRENT_PROBES` für die Anzahl der Geräte und ihre Intervalle zu niedrig ist. |
| `dumbmonit_probes_total{kind}` | Zähler | Gelaufene Abfragen, nach Geräteart (`snmp`, `proxmox`, `http`…). Schließt die ein, die ein Relay-Agent im Auftrag des Servers ausgeführt hat. |
| `dumbmonit_probes_failed_total{kind}` | Zähler | Fehlgeschlagene Abfragen, nach Art — ein nicht erreichbares Gerät oder ein Konfigurationsfehler. `rate(dumbmonit_probes_failed_total[15m]) / rate(dumbmonit_probes_total[15m])` ist die Fehlschlagquote. |
| `dumbmonit_samples_written_total` | Zähler | Von VictoriaMetrics akzeptierte Samples. |
| `dumbmonit_sample_writes_failed_total` | Zähler | Von VictoriaMetrics abgelehnte oder nicht beantwortete Batches. Der Batch bleibt gepuffert und wird wiederholt; ein steigender Wert mit steigendem `dumbmonit_samples_pending` bedeutet, der Store ist down oder voll. |
| `dumbmonit_samples_pending` | Gauge | Samples, die nach dem letzten Flush noch im Schreibpuffer sind. Normalerweise 0. |
| `dumbmonit_samples_dropped_total` | Zähler | Vor Erreichen des Schreibpuffers verworfene Samples, weil er voll oder geschlossen war. Nie wiederholt — ein steigender Wert während eines Ausfalls bedeutet Datenverlust, nicht nur Verzögerung. |
| `dumbmonit_alerting_cycles_total` | Zähler | Gelaufene Alarmierungszyklen (einer alle `DUMBMONIT_ALERT_INTERVAL_SECS`, standardmäßig 30 s). |
| `dumbmonit_alerting_cycle_seconds` | Gauge | Dauer des letzten Alarmierungszyklus, Abfragezeit eingeschlossen. Nähert er sich dem Intervall, fragen die Regeln mehr ab, als der Store bedienen kann. |
| `dumbmonit_alerting_rules_evaluated` | Gauge | Im letzten Zyklus ausgewertete Regeln. |
| `dumbmonit_alerting_rules_failed` | Gauge | Regeln, deren Abfrage im letzten Zyklus fehlschlug. Ihre Alarme sind eingefroren, nicht behoben. Alles außer 0 verdient einen Blick in die Logs. |
| `dumbmonit_alerts{phase}` | Gauge | Von der Engine verfolgte Alarme, nach Phase: `ok`, `pending` (Bedingung wahr, `for` nicht abgelaufen), `firing`, `resolved`. |
| `dumbmonit_alerts_suppressed` | Gauge | Verborgene Alarme, weil ein übergeordnetes Gerät down ist (die effektive Phase `suppressed`). |
| `dumbmonit_alerts_silenced` | Gauge | Von einem Wartungsfenster abgedeckte Alarme. |
| `dumbmonit_alerts_learning` | Gauge | Noch lernende, daher stille Baseline-Alarme. |
| `dumbmonit_notifications_total{kind}` | Zähler | Gesendete Benachrichtigungen, nach Kanalart (`email`, `discord`, `ntfy`…). |
| `dumbmonit_notifications_failed_total{kind}` | Zähler | Vom Kanal abgelehnte Benachrichtigungen, nach Art. Sie werden wieder eingereiht und wiederholt; ein stetig steigender Wert ist ein defekter Webhook oder SMTP-Account. |
| `dumbmonit_agents` | Gauge | Maschinen, die einen Agent registriert haben. |
| `dumbmonit_agents_stale` | Gauge | Agents, die seit fünf Minuten nichts gepusht haben. |
| `dumbmonit_database_up` | Gauge | 1, wenn SQLite antwortet — derselbe Check wie `/api/health`. |
| `dumbmonit_database_bytes` | Gauge | Größe von `dumbmonit.db` und seinem Write-Ahead-Log. Schließt `/data/vm` nicht ein. |
| `dumbmonit_victoriametrics_up` | Gauge | 1, wenn VictoriaMetrics antwortet — derselbe Check wie `/api/health`. 0, während der eingebettete Kindprozess neu startet. |
| `dumbmonit_victoriametrics_embedded` | Gauge | 1, wenn dieser Server VictoriaMetrics selbst betreibt, 0, wenn `DUMBMONIT_VM_URL` auf ein externes zeigt. |

Keine Metrik auf dieser Seite trägt ein Pro-Gerät-Label: Das
Dokument hat dieselbe Größe für ein Gerät und für tausend, und kein
Gerätename sickert zu dem durch, der es abfragen kann.

Zwei Alarme, die sich lohnen auf Ihrer Seite hinzuzufügen, sobald
DumbMonit abgefragt wird:

```
# The monitoring stopped monitoring
up{job="dumbmonit"} == 0

# Measurements are piling up instead of being stored
dumbmonit_samples_pending > 0 and increase(dumbmonit_sample_writes_failed_total[10m]) > 0
```

### Die Routen ohne Token öffnen

`DUMBMONIT_METRICS_PUBLIC=true` bedient `/metrics`, `/federate` und
`/prometheus/api/v1/…` für jeden, der den Port erreichen kann, für
Leute, die den Port selbst filtern — **jeder in diesem Netzwerk kann
dann jede Messung jedes Geräts und den Zustand der Instanz lesen,
setzen Sie es also nur, wenn etwas anderes bereits einschränkt, wer
den Port erreicht.**

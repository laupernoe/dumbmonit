# VictoriaMetrics und VictoriaLogs

Die Zeitreihendatenbank und die Log-Datenbank selbst: ob sie
akzeptieren, was ihnen geschickt wird, wie viel Platte übrig ist,
bevor sie aufhören, irgendetwas zu akzeptieren, und was sie ablehnen.

Zwei Gerätearten teilen sich diese Seite, weil die beiden Produkte ihre
Grundlagen teilen: `victoriametrics` für ein VictoriaMetrics mit einem
Knoten, und `victorialogs` für ein VictoriaLogs mit einem Knoten.
Beide veröffentlichen ihre eigene Gesundheit auf `/health` und
`/metrics`, auf dem Port ihrer API; DumbMonit liest nur diese beiden.
Es führt nie eine Abfrage gegen Ihre Daten aus und schreibt nie.

Diese Server fallen selten ganz aus. Sie fallen *halb* aus: Sie
beantworten weiter Abfragen, und die Dashboards öffnen sich noch,
während jeder neue Sample oder jede Log-Zeile abgelehnt wird, weil die
Platte das Read-Only-Limit überschritten hat oder die Daten mit einem
Zeitstempel außerhalb der Aufbewahrung kamen. Was auch immer zu ihnen
pusht (vmagent, Prometheus Remote Write, Vector, Fluent Bit,
Promtail), versucht es erneut, dann verwirft es. Diese Seite handelt
davon, das kommen zu sehen.

## Was beobachtet wird

Metriken haben das Präfix `dumbmonit_victoriametrics_` oder
`dumbmonit_victorialogs_`. Totals sind Zähler: Diagramme und Regeln
verwandeln sie in Raten.

| Metrik | Was | Labels |
|---|---|---|
| `healthy` | 1, wenn `/health` mit OK antwortet, 0, wenn es anders antwortet | |
| `version_info` | Wert 1 | `version` |
| `rows_ingested_total` | akzeptierte Samples (VictoriaMetrics) oder Log-Zeilen (VictoriaLogs) | |
| `bytes_ingested_total` | akzeptiertes Log-Volumen (nur VictoriaLogs) | |
| `rows_rejected_total` | alles Abgelehnte, immer vorhanden, auch bei null | |
| `rows_rejected_reason_total` | dasselbe nach Grund: `big_timestamp`, `small_timestamp`, `too_long_label_value`, `too_many_labels`, `invalid`, `hourly_series_limit`… (VictoriaMetrics); `too_big_timestamp`, `too_small_timestamp`, `too_many_fields`, `too_long_line` (VictoriaLogs) | `reason` |
| `disk_free_bytes`, `disk_total_bytes` | freier und gesamter Platz des Storage-Volumes | `path` |
| `disk_free_limit_bytes` | `-storage.minFreeDiskSpaceBytes`: darunter wird der Storage read-only | `path` |
| `disk_headroom_percent` | freier Platz über diesem Limit, als Anteil der Platte | `path` |
| `read_only` | 1, sobald der Storage auf read-only gewechselt ist | |
| `data_size_bytes` | Daten auf der Platte | |
| `slow_inserts_total`, `rows_added_total` | neue Samples, die den langsamen Pfad nahmen, und alle hinzugefügten Samples (nur VictoriaMetrics) | |
| `active_series` | Zeitreihen, die in der letzten Stunde einen Sample erhielten (nur VictoriaMetrics) | |
| `http_errors_total` | mit einem Fehler beantwortete Requests | |
| `log_errors_total` | vom Server protokollierte Fehlermeldungen | |
| `uptime_seconds`, `memory_resident_bytes` | | |

**Ein abgelehnter Sample ist verloren.** VictoriaMetrics lehnt einen
Sample ab, der älter als seine Aufbewahrung ist oder mehr als zwei
Tage in der Zukunft liegt, einen Label-Wert länger als
`-maxLabelValueLen`, eine Zeitreihe über `-maxLabelsPerTimeseries`,
und jeden Sample einer neuen Zeitreihe, sobald
`-storage.maxHourlySeries` oder `-storage.maxDailySeries` erreicht
ist. VictoriaLogs lehnt Zeilen außerhalb seiner Aufbewahrung und
Zeilen mit zu vielen Feldern ab. Der Client erhält dafür meist keinen
Fehler: Nur dieser Zähler weiß es.

**Langsame Inserts** sind Samples, deren Zeitreihe VictoriaMetrics auf
der Platte nachschlagen musste statt im Cache. Jede brandneue
Zeitreihe ist eine, daher verursacht ein Deployment einen Schub; ein
Anteil, der über 5 % bleibt, bedeutet, dass dem Server Speicher für
seine aktiven Zeitreihen fehlt. Das ist der von der
VictoriaMetrics-Dokumentation vorgegebene Schwellenwert.

Die geltenden [eingebauten Regeln](../alerting/rules.md#log-and-metrics-servers):
Metrics or log storage read-only, Metrics or log storage almost
read-only, Metrics or log server unhealthy, Metrics or log data
refused, VictoriaMetrics slow inserts, plus Device unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt den Server nie ab. Es sagt, in dieser
Reihenfolge:

1. **Was nicht stimmt, in einem Satz**: Gesundheit, Storage
   (*Read-only*, oder wie viel Platte über dem Limit übrig ist), in
   den letzten 24 Stunden abgelehnte Daten, langsame Inserts,
   protokollierte Fehler. Jede Zeile trägt ein Wort sowie eine Farbe.
2. **Was der Server tut**: Samples oder Zeilen pro Sekunde
   eingenommen, aktive Zeitreihen oder eingenommenes Volumen, Daten
   auf der Platte, Uptime.
3. **Was in den letzten 24 Stunden abgelehnt wurde, nach Grund**, und
   der freie Platz jedes Storage-Pfads gegen sein Read-Only-Limit.

## DumbMonit VictoriaMetrics' eigene Gesundheit lesen lassen

1. Nichts zu installieren: VictoriaMetrics veröffentlicht seine eigene
   Gesundheit auf /health und /metrics, auf dem Port seiner API
   (standardmäßig 8428). Prüfen Sie vom DumbMonit-Host aus, dass beide
   antworten.

    ```
    curl http://victoriametrics.lan:8428/health
    curl -s http://victoriametrics.lan:8428/metrics | grep vm_app_version
    ```

2. Läuft VictoriaMetrics mit -httpAuth.username und
   -httpAuth.password, wählen Sie unten Benutzername / Passwort und
   geben Sie diese ein. Hinter vmauth oder einem Reverse Proxy, der
   ein Bearer-Token erwartet, wählen Sie Bearer Token. Ist
   -metricsAuthKey gesetzt, antwortet /metrics nur mit diesem Key in
   der URL, den DumbMonit nicht sendet: Schützen Sie /metrics
   stattdessen mit der obigen Basic-Authentifizierung.

3. Geben Sie in DumbMonit die Serveradresse ein, zum Beispiel
   "victoriametrics.lan" oder "http://10.0.0.5:8428". Hinter einem
   Reverse Proxy geben Sie die vollständige URL mit ihrem Pfadpräfix
   ein. Dies deckt den Server mit einem Knoten ab; die Komponenten
   eines VictoriaMetrics-Clusters stellen andere Metriken bereit und
   werden nicht abgedeckt.

4. DumbMonit liest nur /health und /metrics. Es führt nie eine Abfrage
   gegen Ihre Daten aus und schreibt nie etwas.

!!! warning
    Freier Plattenplatz ist, was zu beobachten ist: unter
    -storage.minFreeDiskSpaceBytes (standardmäßig 100 MB) wechselt
    VictoriaMetrics auf read-only und lehnt jeden neuen Sample ab,
    während es weiter Abfragen beantwortet, als wäre nichts
    geschehen. DumbMonit zeigt den verbleibenden Spielraum über diesem
    Limit und warnt unter 10 %.

## DumbMonit VictoriaLogs' eigene Gesundheit lesen lassen

1. Nichts zu installieren: VictoriaLogs veröffentlicht seine eigene
   Gesundheit auf /health und /metrics, auf dem Port seiner API
   (standardmäßig 9428). Prüfen Sie vom DumbMonit-Host aus, dass beide
   antworten.

    ```
    curl http://victorialogs.lan:9428/health
    curl -s http://victorialogs.lan:9428/metrics | grep vl_rows_ingested_total
    ```

2. Läuft VictoriaLogs mit -httpAuth.username und -httpAuth.password,
   wählen Sie unten Benutzername / Passwort und geben Sie diese ein.
   Hinter vmauth oder einem Reverse Proxy, der ein Bearer-Token
   erwartet, wählen Sie Bearer Token. Ist -metricsAuthKey gesetzt,
   antwortet /metrics nur mit diesem Key in der URL, den DumbMonit
   nicht sendet: Schützen Sie /metrics stattdessen mit der obigen
   Basic-Authentifizierung.

3. Geben Sie in DumbMonit die Serveradresse ein, zum Beispiel
   "victorialogs.lan" oder "http://10.0.0.6:9428". Hinter einem
   Reverse Proxy geben Sie die vollständige URL mit ihrem Pfadpräfix
   ein.

4. DumbMonit liest nur /health und /metrics. Es führt nie eine
   LogsQL-Abfrage aus und liest nie eine Log-Zeile.

!!! warning
    Freier Plattenplatz ist, was zu beobachten ist: unter
    -storage.minFreeDiskSpaceBytes (standardmäßig 10 MB) wechselt
    VictoriaLogs auf read-only und lehnt jede neue Log-Zeile ab.
    DumbMonit zeigt den verbleibenden Spielraum über diesem Limit und
    warnt unter 10 %.

## Credentials

| Credential | Felder |
|---|---|
| Keine Authentifizierung | Der Standard: `/health` und `/metrics` sind offen, sofern Sie sie nicht geschützt haben. |
| Benutzername / Passwort | Die `-httpAuth.username` und `-httpAuth.password` des Servers, oder des vorgeschalteten Reverse Proxy. |
| Bearer Token | Ein Token, das vmauth oder Ihr Reverse Proxy in `Authorization: Bearer` erwartet. |

Adresse: ein Hostname oder eine IP (`victoriametrics.lan`,
`10.0.0.5`), `host:port`, oder eine vollständige URL mit Pfadpräfix
hinter einem Reverse Proxy (`https://metrics.lan/vm`). Optionen:
Protokoll (standardmäßig HTTP), Port (8428 oder 9428),
Zertifikatsprüfung und Anfrage-Timeout (10 s).

# VictoriaMetrics und VictoriaLogs {#victoriametrics-and-victorialogs}

Die Zeitreihendatenbank und die Logdatenbank selbst: ob sie annehmen, was ihnen gesendet wird,
wie viel Plattenplatz bleibt, bevor sie nichts mehr annehmen, und was sie ablehnen.

Zwei Monitortypen teilen sich diese Seite, weil die beiden Produkte dieselben Grundlagen haben:
`victoriametrics` für ein Single-Node-VictoriaMetrics und `victorialogs` für ein
Single-Node-VictoriaLogs. Beide veröffentlichen ihren eigenen Zustand auf `/health` und
`/metrics`, auf dem Port ihrer API; DumbMonit liest diese beiden und sonst nichts. Es führt nie
eine Abfrage gegen Ihre Daten aus und schreibt nie.

Diese Server fallen selten ganz aus. Sie fallen *zur Hälfte* aus: Sie beantworten weiter
Abfragen und die Dashboards öffnen sich noch, während jeder neue Messwert oder jede neue
Logzeile abgelehnt wird, weil die Platte die Read-only-Grenze unterschritten hat oder die Daten
mit einem Zeitstempel außerhalb der Aufbewahrung kamen. Was auch immer dorthin sendet (vmagent,
Prometheus Remote Write, Vector, Fluent Bit, Promtail), versucht es erneut und verwirft dann.
Auf dieser Seite geht es darum, das kommen zu sehen.

## Was überwacht wird {#what-it-watches}

Metriken haben das Präfix `dumbmonit_victoriametrics_` oder `dumbmonit_victorialogs_`. Summen
sind Zähler: Diagramme und Regeln wandeln sie in Raten um.

| Metrik | Was | Labels |
|---|---|---|
| `healthy` | 1, wenn `/health` mit OK antwortet, 0, wenn mit etwas anderem | |
| `version_info` | Wert 1 | `version` |
| `rows_ingested_total` | angenommene Messwerte (VictoriaMetrics) oder Logzeilen (VictoriaLogs) | |
| `bytes_ingested_total` | angenommenes Logvolumen (nur VictoriaLogs) | |
| `rows_rejected_total` | alles Abgelehnte, immer vorhanden, auch bei null | |
| `rows_rejected_reason_total` | dasselbe, nach Grund: `big_timestamp`, `small_timestamp`, `too_long_label_value`, `too_many_labels`, `invalid`, `hourly_series_limit` … (VictoriaMetrics); `too_big_timestamp`, `too_small_timestamp`, `too_many_fields`, `too_long_line` (VictoriaLogs) | `reason` |
| `disk_free_bytes`, `disk_total_bytes` | freier und gesamter Platz des Speichervolumes | `path` |
| `disk_free_limit_bytes` | `-storage.minFreeDiskSpaceBytes`: darunter wird der Speicher schreibgeschützt | `path` |
| `disk_headroom_percent` | über dieser Grenze verbleibender freier Platz als Anteil der Platte | `path` |
| `read_only` | 1, sobald der Speicher auf Read-only umgeschaltet hat | |
| `data_size_bytes` | Daten auf der Platte | |
| `slow_inserts_total`, `rows_added_total` | neue Messwerte auf dem langsamen Pfad und alle hinzugefügten Messwerte (nur VictoriaMetrics) | |
| `active_series` | Serien, die in der letzten Stunde einen Messwert erhielten (nur VictoriaMetrics) | |
| `http_errors_total` | mit einem Fehler beantwortete Anfragen | |
| `log_errors_total` | vom Server protokollierte Fehlermeldungen | |
| `uptime_seconds`, `memory_resident_bytes` | | |

**Ein abgelehnter Messwert ist verloren.** VictoriaMetrics lehnt einen Messwert ab, der älter als
seine Aufbewahrung oder mehr als zwei Tage in der Zukunft ist, einen Label-Wert, der länger als
`-maxLabelValueLen` ist, eine Serie über `-maxLabelsPerTimeseries` und jeden Messwert einer
neuen Serie, sobald `-storage.maxHourlySeries` oder `-storage.maxDailySeries` erreicht ist.
VictoriaLogs lehnt Zeilen außerhalb seiner Aufbewahrung und Zeilen mit zu vielen Feldern ab. Der
Client erhält dafür meist keinen Fehler: Nur dieser Zähler weiß davon.

**Slow Inserts** sind Messwerte, deren Serie VictoriaMetrics auf der Platte statt in seinem Cache
nachschlagen musste. Jede brandneue Serie ist einer, daher verursacht ein Deployment eine Spitze;
ein Anteil, der über 5 % bleibt, bedeutet, dass dem Server der Speicher für seine aktiven Serien
fehlt. Das ist der Schwellenwert, den die VictoriaMetrics-Dokumentation nennt. Nach einem
Neustart sind die Caches des Servers kalt, daher kann der Anteil eine Weile hoch liegen: Die
Regel wartet eine halbe Stunde, bevor sie auslöst. Zähler beginnen mit dem Server wieder bei
null; die Regeln verwenden Anstiege, die Resets berücksichtigen, sodass ein Neustart nicht als
abgelehnte Daten zählt.

Die zutreffenden [integrierten Regeln](../alerting/rules.md#log-and-metrics-servers):
Metrik- oder Logspeicher schreibgeschützt, Metrik- oder Logspeicher fast schreibgeschützt, Metrik-
oder Logserver fehlerhaft, Metrik- oder Logdaten abgelehnt, VictoriaMetrics Slow Inserts,
außerdem Monitor nicht erreichbar.

## Die Monitorseite {#the-device-page}

Das Panel über den Diagrammen liest, was die Prüfung gespeichert hat; das Öffnen der Seite fragt
den Server nie ab. Es nennt, in dieser Reihenfolge:

1. **Was nicht stimmt, in einem Satz**: Zustand, Speicher (*Read-only* oder wie viel Platz über
   der Grenze verbleibt), in den letzten 24 Stunden abgelehnte Daten, Slow Inserts,
   protokollierte Fehler. Jede Zeile trägt neben einer Farbe auch ein Wort.
2. **Was der Server tut**: pro Sekunde angenommene Messwerte oder Zeilen, aktive Serien oder
   angenommenes Volumen, Daten auf der Platte, Laufzeit.
3. **Was in den letzten 24 Stunden abgelehnt wurde, nach Grund**, und den freien Platz jedes
   Speicherpfads gegenüber seiner Read-only-Grenze.

## DumbMonit den eigenen Zustand von VictoriaMetrics lesen lassen {#let-dumbmonit-read-victoriametrics-own-health}

1. Nichts zu installieren: VictoriaMetrics veröffentlicht seinen eigenen Zustand auf /health und /metrics, auf dem Port seiner API (standardmäßig 8428). Prüfen Sie vom DumbMonit-Host aus, dass beide antworten.

    ```
    curl http://victoriametrics.lan:8428/health
    curl -s http://victoriametrics.lan:8428/metrics | grep vm_app_version
    ```

2. Läuft VictoriaMetrics mit -httpAuth.username und -httpAuth.password, wählen Sie unten Benutzername / Passwort und geben Sie diese ein. Hinter vmauth oder einem Reverse-Proxy, der ein Bearer-Token erwartet, wählen Sie Bearer-Token. Ist -metricsAuthKey gesetzt, antwortet /metrics nur mit diesem Schlüssel in der URL, den DumbMonit nicht sendet: Schützen Sie /metrics stattdessen mit der obigen Basisauthentifizierung.

3. Geben Sie in DumbMonit die Serveradresse ein, zum Beispiel „victoriametrics.lan“ oder „http://10.0.0.5:8428“. Hinter einem Reverse-Proxy geben Sie die vollständige URL mit ihrem Pfadpräfix ein. Das gilt für den Single-Node-Server; die Komponenten eines VictoriaMetrics-Clusters stellen andere Metriken bereit und werden nicht abgedeckt.

4. DumbMonit liest nur /health und /metrics. Es führt nie eine Abfrage gegen Ihre Daten aus und schreibt nie etwas.

!!! warning
    Zu überwachen ist der freie Plattenplatz: Unter -storage.minFreeDiskSpaceBytes (standardmäßig 100 MB) schaltet VictoriaMetrics auf Read-only und lehnt jeden neuen Messwert ab, während es Abfragen weiter beantwortet, als wäre nichts. DumbMonit zeigt den über dieser Grenze verbleibenden Spielraum und warnt unter 10 %.

## DumbMonit den eigenen Zustand von VictoriaLogs lesen lassen {#let-dumbmonit-read-victorialogs-own-health}

1. Nichts zu installieren: VictoriaLogs veröffentlicht seinen eigenen Zustand auf /health und /metrics, auf dem Port seiner API (standardmäßig 9428). Prüfen Sie vom DumbMonit-Host aus, dass beide antworten.

    ```
    curl http://victorialogs.lan:9428/health
    curl -s http://victorialogs.lan:9428/metrics | grep vl_free_disk_space_bytes
    ```

2. Läuft VictoriaLogs mit -httpAuth.username und -httpAuth.password, wählen Sie unten Benutzername / Passwort und geben Sie diese ein. Hinter vmauth oder einem Reverse-Proxy, der ein Bearer-Token erwartet, wählen Sie Bearer-Token. Ist -metricsAuthKey gesetzt, antwortet /metrics nur mit diesem Schlüssel in der URL, den DumbMonit nicht sendet: Schützen Sie /metrics stattdessen mit der obigen Basisauthentifizierung.

3. Geben Sie in DumbMonit die Serveradresse ein, zum Beispiel „victorialogs.lan“ oder „http://10.0.0.6:9428“. Hinter einem Reverse-Proxy geben Sie die vollständige URL mit ihrem Pfadpräfix ein.

4. DumbMonit liest nur /health und /metrics. Es führt nie eine LogsQL-Abfrage aus und liest nie eine Logzeile.

!!! warning
    Zu überwachen ist der freie Plattenplatz: Unter -storage.minFreeDiskSpaceBytes (standardmäßig 10 MB) schaltet VictoriaLogs auf Read-only und lehnt jede neue Logzeile ab. DumbMonit zeigt den über dieser Grenze verbleibenden Spielraum und warnt unter 10 %.

## Zugangsdaten {#credentials}

| Zugangsdaten | Felder |
|---|---|
| Keine Authentifizierung | Der Standard: `/health` und `/metrics` sind offen, sofern Sie sie nicht geschützt haben. |
| Benutzername / Passwort | Das `-httpAuth.username` und `-httpAuth.password` des Servers oder des Reverse-Proxys davor. |
| Bearer-Token | Ein Token, das vmauth oder Ihr Reverse-Proxy in `Authorization: Bearer` erwartet. |

Adresse: ein Hostname oder eine IP (`victoriametrics.lan`, `10.0.0.5`), `host:port` oder eine
vollständige URL mit Pfadpräfix hinter einem Reverse-Proxy (`https://metrics.lan/vm`). Optionen:
Protokoll (standardmäßig HTTP), Port (8428 oder 9428), Zertifikatsprüfung und Anfrage-Timeout
(10 s).

# Grafana Loki

Der Log-Server selbst: ob er bereit ist, welche Log-Zeilen er ablehnt und
warum, ob seine Chunks den Storage erreichen, und ob sein
Write-Ahead-Log noch Plattenplatz hat.

Loki veröffentlicht `/ready` und `/metrics` auf seinem HTTP-Port;
DumbMonit liest nur diese beiden. Es führt nie eine LogQL-Abfrage aus und
liest nie eine Log-Zeile.

Der beobachtenswerte Fehler ist leise. Wenn Loki eine Zeile ablehnt
(älter als `reject_old_samples_max_age`, über der Ingestionsrate oder
dem Stream-Limit des Mandanten, länger als `max_line_size`), erhält der
Sender ein 4xx, und Promtail, Alloy oder Fluent Bit verwerfen meist den
Batch. Grafana zeigt nichts außer einer Lücke, gefunden an dem Tag, an
dem jemand nach dem Log eines Vorfalls sucht. Der zweite ist, dass
Chunks nicht in den Storage geschrieben werden können: Loki hält sie im
Speicher und verliert sie beim nächsten Neustart.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_loki_`. Totals sind Zähler:
Diagramme und Regeln verwandeln sie in Raten.

| Metrik | Was | Labels |
|---|---|---|
| `ready` | 1, wenn `/ready` mit `ready` antwortet, 0, während Loki startet oder wenn eine Komponente nicht schreiben oder lesen kann | |
| `version_info` | Wert 1 | `version` |
| `received_lines_total`, `received_bytes_total` | was der Distributor empfangen hat, alle Mandanten | |
| `discarded_lines_total` | abgelehnte Zeilen, immer vorhanden, auch bei null | |
| `discarded_lines_reason_total` | dasselbe nach Grund: `greater_than_max_sample_age`, `rate_limited`, `stream_limit`, `line_too_long`, `per_stream_rate_limit`… | `reason` |
| `flush_failures_total`, `flush_queue_length` | Chunks, die den Storage nicht erreicht haben, wartende Chunks | |
| `memory_streams` | vom Ingester im Speicher gehaltene Streams | |
| `wal_disk_full_failures_total`, `wal_disk_used_percent` | an einer vollen Platte verlorene Write-Ahead-Log-Schreibvorgänge, und wie voll diese Platte ist | |
| `requests_total`, `request_errors_total` | HTTP- und gRPC-Requests, und die mit einem 5xx oder einem gRPC-Fehler beantworteten. Ein 4xx ist Schuld des Clients und wird nicht als Fehler gezählt. | |
| `ring_unhealthy_members` | als ungesund markierte Mitglieder der Hash-Ringe | |
| `panics_total`, `log_errors_total` | aufgefangene Panics, protokollierte Fehlermeldungen | |
| `retention_last_run_timestamp_seconds` | letzter erfolgreicher Retention-Lauf des Compactors, wenn Retention an ist | |
| `memory_resident_bytes` | | |

Ein in Komponenten aufgeteiltes Loki (simple scalable, microservices)
stellt diese Familien dort aus, wo sie hingehören: abgelehnte Zeilen am
Distributor, Flush und WAL am Ingester. Fügen Sie die zu beobachtenden
Komponenten hinzu; eine Familie, die eine Komponente nicht hat, erzeugt
einfach keine Zeitreihe.

Die geltenden [eingebauten Regeln](../alerting/rules.md#log-and-metrics-servers):
Loki not ready, Loki refusing log lines, Loki cannot flush to storage,
Loki write-ahead log disk full, Loki request errors, plus Monitor
unreachable.

## Die Monitorseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt Loki nie ab. Es sagt in einem Satz, was nicht
stimmt (Bereitschaft, in den letzten 24 Stunden abgelehnte Zeilen,
nicht geflushte Chunks, eine volle WAL-Platte, ungesunde Ring-Mitglieder,
Serverfehler), dann die empfangenen Zeilen und das Volumen pro Sekunde,
die Streams im Speicher und die WAL-Platte, dann die in den letzten 24
Stunden abgelehnten Zeilen nach Grund.

## DumbMonit Lokis eigene Gesundheit lesen lassen

1. Nichts zu installieren: Loki veröffentlicht /ready und /metrics auf
   seinem HTTP-Port (standardmäßig 3100). Prüfen Sie vom DumbMonit-Host
   aus, dass beide antworten.

    ```
    curl http://loki.lan:3100/ready
    curl -s http://loki.lan:3100/metrics | grep loki_build_info
    ```

2. Loki hat keinen eigenen Login. Sitzt es hinter einem Reverse Proxy,
   der Benutzername und Passwort verlangt, wählen Sie unten
   Benutzername / Passwort; für ein Bearer-Token wählen Sie Bearer
   Token. Kein Mandant ist nötig: /ready und /metrics sind nicht pro
   Mandant.

3. Geben Sie in DumbMonit Lokis Adresse ein, zum Beispiel "loki.lan",
   oder "https://logs.lan/loki" hinter einem Reverse Proxy. Bei einem in
   Komponenten aufgeteilten Loki (simple scalable oder microservices)
   fügen Sie mindestens den Write-Pfad hinzu, wo abgelehnte Zeilen und
   Flush-Fehler gezählt werden.

4. DumbMonit liest nur /ready und /metrics. Es führt nie eine
   LogQL-Abfrage aus und liest nie eine Log-Zeile.

!!! warning
    Abgelehnte Zeilen sind der zu beobachtende Fehler: Wenn Loki eine
    Zeile ablehnt (zu alt, über der Ingestionsrate oder dem
    Stream-Limit, Zeile zu lang), erhält der Sender einen Fehler und
    verwirft meist den Batch. In Grafana zeigt sich nichts außer einer
    Lücke in den Logs.

## Credentials

| Credential | Felder |
|---|---|
| Keine Authentifizierung | Der Standard: Loki selbst hat keinen Login. |
| Benutzername / Passwort | Die Basic-Authentifizierung des vorgeschalteten Reverse Proxy. |
| Bearer Token | Ein Token, das Ihr Reverse Proxy in `Authorization: Bearer` erwartet. |

Adresse: ein Hostname oder eine IP (`loki.lan`), `host:port`, oder eine
vollständige URL mit Pfadpräfix (`https://logs.lan/loki`). Optionen:
Protokoll (standardmäßig HTTP), Port (3100), Zertifikatsprüfung und
Anfrage-Timeout (10 s).

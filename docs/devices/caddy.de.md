# Caddy

Der Webserver und Reverse Proxy: Upstreams, die ihren Healthcheck nicht
bestehen, ein fehlgeschlagenes Konfigurations-Reload, Handler-Fehler und
der Anteil der 5xx-Antworten.

DumbMonit liest drei Dinge aus Caddys Admin-API, alle mit `GET`:
`/config/apps/http/servers` (die von Caddy ausgeführte HTTP-Konfiguration,
nie den `tls`-Abschnitt, in dem DNS-Provider-Tokens liegen),
`/reverse_proxy/upstreams` (die Backends und die vom passiven Healthcheck
gemerkten Fehler) und `/metrics` (Upstream-Gesundheit, Reloads, Requests
und Fehler).

Caddys Admin-API ist nicht nur lesend: Wer sie erreicht, kann eine neue
Konfiguration laden oder Caddy stoppen. Aus diesem Grund hört sie auf
`localhost:2019`. Die Einrichtung unten lässt sie dort und lässt Caddy
selbst eine nur-lesende, passwortgeschützte Ansicht davon veröffentlichen,
die auf `GET` auf den drei obigen Pfaden antwortet und alles andere
abweist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_caddy_`. Totals sind Zähler:
Diagramme und Regeln verwandeln sie in Raten.

| Metrik | Was | Labels |
|---|---|---|
| `config_loaded` | 1, wenn Caddy mindestens einen HTTP-Server betreibt | |
| `servers`, `routes` | HTTP-Server und Routen in der Konfiguration | |
| `config_last_reload_ok` | 0, wenn das letzte Konfigurations-Reload fehlgeschlagen ist | |
| `config_last_reload_timestamp_seconds` | Zeitpunkt des letzten erfolgreichen Reloads | |
| `upstreams`, `upstreams_unhealthy` | Backends jedes `reverse_proxy`, und die fehlschlagenden | |
| `upstream_healthy` | 1, wenn weder der aktive noch der passive Healthcheck das Backend ausschließt | `upstream` |
| `upstream_fails` | fehlgeschlagene Requests, die sich der passive Healthcheck aktuell merkt | `upstream` |
| `upstream_requests` | laufende Requests zum Backend | `upstream` |
| `requests_total`, `requests_5xx_total`, `request_errors_total` | mit der `metrics`-Option: Requests, 5xx-Antworten und Handler-Fehler | |
| `server_requests_total`, `server_requests_5xx_total`, `server_request_errors_total` | dasselbe pro Server (`srv0`, `srv1`… oder die von Ihnen vergebenen Namen) | `server` |

`upstream_healthy` braucht `/metrics`: Der aktive Healthcheck ist nur dort
sichtbar. Eine nur-lesende Ansicht, die `/metrics` nicht durchlässt,
meldet trotzdem die Backends und ihre gemerkten Fehler. Caddy
veröffentlicht seine Version nicht über die Admin-API.

Die geltenden [eingebauten Regeln](../alerting/rules.md#reverse-proxies-and-domains):
Caddy upstream unhealthy, Caddy configuration reload failed, Reverse
proxy answering 5xx, plus Device unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt Caddy nie ab. Es sagt in einem Satz, was nicht
stimmt (keine Konfiguration, ein fehlgeschlagenes Reload, ungesunde
Upstreams, ein hoher 5xx-Anteil), dann die Server, Routen, Upstreams,
Request-Rate, den 5xx-Anteil und die Handler-Fehler über 24 Stunden, dann
jeden Upstream mit seinen gemerkten Fehlern.

## Eine nur-lesende Ansicht von Caddys Admin-API veröffentlichen

1. Caddys Admin-API kann eine neue Konfiguration laden und Caddy stoppen:
   Sie muss außer Reichweite von allem bleiben außer DumbMonit. Lassen Sie
   sie, wo Caddy sie hinlegt, auf localhost:2019, und lassen Sie Caddy
   eine nur-lesende Ansicht davon veröffentlichen: nur GET, auf den drei
   von DumbMonit gelesenen Pfaden, hinter einem Passwort.

2. Erzeugen Sie den Passwort-Hash für den Benutzer dumbmonit.

    ```
    caddy hash-password --plaintext 'a-long-password'
    ```

3. Fügen Sie diese Site mit diesem Hash an Stelle von HASH zum Caddyfile
   hinzu, laden Sie Caddy dann neu. Sie antwortet auf Port 2020 und weist
   alles außer diesen drei Lesevorgängen ab.

    ```
    :2020 {
    basic_auth {
    dumbmonit HASH
    }
    @readonly {
    method GET
    path /config/apps/http/servers /reverse_proxy/upstreams /metrics
    }
    handle @readonly {
    reverse_proxy localhost:2019 {
    header_up Host localhost:2019
    }
    }
    respond 403
    }
    ```

4. Optional: Fügen Sie `metrics` dem globalen Options-Block am Anfang des
   Caddyfiles hinzu, um Requests, Fehler und 5xx-Antworten pro Server zu
   zählen. Upstream-Gesundheit und Reloads werden auch ohne das gemeldet.

    ```
    metrics
    ```

5. Filtern Sie Port 2020, sodass nur der DumbMonit-Host ihn erreicht. In
   DumbMonit geben Sie den Caddy-Host mit diesem Port ein, zum Beispiel
   "caddy.lan:2020", und den Benutzer dumbmonit mit seinem Passwort.

!!! warning
    Veröffentlichen Sie niemals die Admin-API selbst im Netzwerk ohne eine
    vorgeschaltete Firewall: Wer Port 2019 erreicht, kann die
    Konfiguration ersetzen oder Caddy stoppen, und die
    Origins-Option stoppt keinen Client, der keinen Origin-Header
    sendet. Wenn Sie sie doch in einem privaten Netzwerk exponieren,
    filtern Sie den Port, sodass nur der DumbMonit-Host ihn erreicht, und
    wählen Sie Kein Passwort.

Das Caddyfile behält seinen Sinn ohne Einrückung; `caddy fmt` rückt es
ein. `header_up Host localhost:2019` wird benötigt, weil die Admin-API
nur Requests beantwortet, die an die Adresse adressiert sind, auf der sie
hört. Mit dem Caddy-Container-Image führen Sie den Hash-Befehl mit
`docker exec` aus.

Eine als JSON statt aus einem Caddyfile geladene Konfiguration erhält
dieselbe Site als zusätzlichen Server mit einem `basic_auth`-Handler,
einem `method`- und `path`-Matcher und einem `reverse_proxy` zu
`localhost:2019`.

## Credentials

| Credential | Felder |
|---|---|
| Nur-lesende Ansicht | Benutzer und Passwort des obigen `basic_auth`-Blocks (empfohlen). |
| Kein Passwort | Nichts: die Admin-API selbst, gefiltert, sodass nur DumbMonit sie erreicht. |

Adresse: ein Hostname oder eine IP mit dem Port der nur-lesenden Ansicht
(`caddy.lan:2020`), oder eine vollständige URL. Optionen: Protokoll
(standardmäßig HTTP), Port (2019, die Admin-API selbst),
Zertifikatsprüfung und Anfrage-Timeout (10 s).

Der passive Healthcheck (`fail_duration`) merkt sich Fehler nur für diese
Dauer: Ein Backend, das einmal fehlschlägt und sich erholt, wird bis dahin
als ungesund gemeldet. Die Regel wartet fünf Minuten, bevor sie feuert.

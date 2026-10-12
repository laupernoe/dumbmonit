# Nginx

Der Webserver und Reverse Proxy: aktive und wartende Verbindungen, die
Accept-/Handled-/Request-Raten, und Upstream-Gesundheit, wenn die
kommerzielle NGINX-Plus-API verfügbar ist.

DumbMonit liest Nginx' eingebautes Modul `stub_status`, eine Handvoll
Klartext-Zähler, ohne API und ohne Konto nötig auf einem korrekt
gefilterten Netzwerk. Wenn das kommerzielle NGINX Plus installiert ist,
kann DumbMonit zusätzlich dessen JSON-API für Upstream-Gesundheit und
Zonen-Zähler lesen.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_nginx_`. Die `_total`-Metriken
sind Zähler: Diagramme und Regeln verwandeln sie in Raten.

| Metrik | Was | Labels |
|---|---|---|
| `connections_active` | derzeit offene Verbindungen | |
| `connections_reading`, `connections_writing`, `connections_waiting` | Verbindungen nach dem, was sie tun | |
| `accepts_total`, `handled_total` | angenommene Verbindungen, und tatsächlich bediente | |
| `requests_total` | seit Nginx-Start bediente Requests | |
| `version_info` | Vorhandensein; der `Server`-Antwort-Header, wenn Nginx einen sendet | `server` |
| `plus_upstream_server_up` | 1, wenn NGINX Plus den Server als up markiert | `upstream`, `server` |
| `plus_upstream_server_active`, `plus_upstream_server_requests_total`, `plus_upstream_server_fails_total`, `plus_upstream_server_5xx_total` | NGINX-Plus-Zähler pro Server | `upstream`, `server` |
| `plus_zone_requests_total`, `plus_zone_5xx_total`, `plus_zone_received_bytes_total`, `plus_zone_sent_bytes_total` | NGINX-Plus-Zähler pro Zone | `zone` |

Eine Lücke zwischen `accepts_total` und `handled_total` bedeutet
Verbindungen, die Nginx verworfen hat, bevor es sie bedienen konnte
(meist weil `worker_connections` erreicht wurde). Die `plus_*`-Metriken
erscheinen nur, wenn "Read the NGINX Plus API" angekreuzt ist und NGINX
Plus antwortet; der Open-Source-Build hat kein "Upstream", über das es
allein mit `stub_status` berichten könnte.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt Nginx nie ab. Es zeigt aktive, lesende, schreibende
und wartende Verbindungen, die Accept-/Handled-/Request-Raten über 24
Stunden, und, mit NGINX Plus, Upstream-Gesundheit und Traffic pro Zone.

## Nginx' stub_status-Modul aktivieren

1. Nginx liefert stub_status eingebaut mit: Es braucht nur eine
   Location, die es einschaltet. Fügen Sie dies dem Server-Block hinzu,
   den Nginx schon bedient (oder einem eigenen Server-Block auf einem
   eigenen Port), dann Nginx neu laden.

    ```
    location /basic_status {
    stub_status;
    }
    ```

2. Beschränken Sie diese Location auf den DumbMonit-Host, sodass niemand
   sonst sie lesen kann; passen Sie die Adresse an.

    ```
    location /basic_status {
    stub_status;
    allow 10.0.0.5;
    deny all;
    }
    nginx -s reload
    ```

3. Prüfen Sie, dass sie antwortet, bevor DumbMonit konfiguriert wird.

    ```
    curl -s http://nginx.lan/basic_status
    ```

4. Geben Sie in DumbMonit den Nginx-Host ein, zum Beispiel "nginx.lan",
   und den Pfad, falls er nicht "/basic_status" ist. Ist NGINX Plus
   installiert, kreuzen Sie "Read the NGINX Plus API" an, um
   Upstream-Gesundheit und Zonen-Zähler hinzuzufügen.

!!! warning
    stub_status zählt nur Verbindungen und Requests: Ein
    Konfigurationsfehler in einem Server-Block, oder ein Backend, das
    nginx nicht erreichen kann, zeigt sich hier nicht. Der
    Open-Source-Build hat keinen Begriff von "Upstream": Das gibt es nur
    in NGINX Plus, über dessen eigene API gelesen, wenn aktiviert.

Das obige Beispiel beschränkt die Location per IP, die einfachste
Einrichtung für ein Homelab; ein `basic_auth`-Block funktioniert genauso
gut, wenn die Statusseite nicht hinter einer eigenen Firewall-Regel
sitzen kann. `stub_status` selbst kann von Nginx nicht direkt mit einem
Passwort geschützt werden — das Passwort, falls vorhanden, gehört zum
umgebenden `location`-Block.

## Credentials

| Credential | Felder |
|---|---|
| Kein Passwort | Nichts: Die Statusseite ist so gefiltert, dass nur DumbMonit sie erreicht. |
| Basic auth | Benutzer und Passwort des Blocks, der die Statusseite schützt, falls vorhanden. |

Adresse: ein Hostname oder eine IP, mit dem Port, falls er nicht 80 ist.
Optionen: Protokoll (standardmäßig HTTP), Port, Status-Pfad
(standardmäßig `/basic_status`), Zertifikatsprüfung, Anfrage-Timeout
(10 s), sowie der Schalter und die Version der NGINX-Plus-API.

# Traefik

Der Reverse Proxy: durch einen Konfigurationsfehler deaktivierte
Router, Backend-Server, die ihren Healthcheck nicht bestehen,
Zertifikate, die ein Resolver nie erhalten hat oder die nicht erneuert
werden, und der Anteil der 5xx-Antworten.

DumbMonit liest Traefiks API, die nur liest: `/api/overview`,
`/api/http/routers`, `/api/http/services`, `/api/certificates`
(bedient von Traefik 3.7) und `/api/version`. Wenn Traefiks
Prometheus-Metriken auf demselben Port bedient werden, fügt
`/metrics` die Request-Rate und den Anteil der 5xx-Antworten hinzu.
Keine Methode der API ändert etwas, und DumbMonit sendet keine.

Die beobachtenswerten Fehler sind die stillen. Ein Router, dessen
Service oder Middleware nicht existiert, oder dessen Regel nicht
parst, ist deaktiviert: seine Sites antworten 404, und Traefik sagt
das nur in seinem Log. Ein Service, dessen Server alle ihren
Healthcheck nicht bestehen, antwortet 503 "no available server". Ein
Router, der einen ACME-Resolver nach einem Zertifikat fragt, das der
Resolver nicht erhalten kann, erhält Traefiks selbstsigniertes
Standardzertifikat, das Browser ablehnen.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_traefik_`. Totals sind
Zähler: Diagramme und Regeln verwandeln sie in Raten.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version` |
| `routers`, `routers_errors`, `routers_warnings` | Router jedes Protokolls, und die im Fehler- oder Warnzustand | |
| `services`, `services_errors`, `middlewares_errors` | dasselbe für Services und Middlewares | |
| `router_status` | pro HTTP-Router: 0 enabled, 1 warning, 2 disabled | `router`, `provider` |
| `router_error_info` | Wert 1, für einen nicht aktivierten Router: Traefiks erster Fehler | `router`, `error` |
| `service_status` | pro Service: 0 enabled, 1 warning, 2 disabled | `service` |
| `service_servers`, `service_servers_up` | Server eines lastverteilten Services, und die ihren Healthcheck bestehenden | `service` |
| `server_up` | 1, wenn der Server seinen Healthcheck besteht (oder keinen hat) | `service`, `server` |
| `resolver_routers` | Router, die einen Certificate-Resolver nach ihrem Zertifikat fragen | `resolver` |
| `resolver_routers_uncovered` | die, deren Hostnamen noch kein von Traefik gehaltenes Zertifikat abdeckt (braucht `/api/certificates`) | `resolver` |
| `certificates`, `cert_expiry_days` | von Traefik gehaltene Zertifikate, und die verbleibenden Tage bis jedes abläuft | `cert` |
| `requests_total`, `requests_5xx_total` | mit Metriken: Requests jedes Entry Points, und die mit 5xx beantworteten | |
| `router_requests_total`, `router_requests_5xx_total` | mit Metriken und Router-Labels: dasselbe pro Router | `router` |
| `config_reloads_total` | mit Metriken: Konfigurations-Reloads | |

Traefiks eigene Router und Services (`@internal`) werden ausgelassen.
Router, Services und Server werden einzeln bis zu je 500 beschrieben;
die Gesamtzahlen decken immer alles ab.

Die geltenden [eingebauten Regeln](../alerting/rules.md#reverse-proxies-and-domains):
Traefik router disabled, Traefik backend server down, Traefik service
without a server, Traefik certificate not obtained, Proxy certificate
not renewed, Reverse proxy answering 5xx, plus Device unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt Traefik nie ab. Es sagt in einem Satz, was
nicht stimmt (deaktivierte Router und Traefiks Grund, Services ohne
Server, Resolver, die nichts erhalten haben, kurz vor dem Ablauf
stehende Zertifikate, ein hoher 5xx-Anteil), dann die Router, Services,
Request-Rate und den 5xx-Anteil, dann die Server up pro Service, die
Zertifikate, die am ehesten ablaufen, zuerst, und die 5xx-Antworten
pro Router über 24 Stunden.

## DumbMonit Traefiks API lesen lassen

1. Traefiks API liest nur: Router, Services und Zertifikate, nie eine
   Änderung. Aktivieren Sie sie mit den Prometheus-Metriken, die die
   Request-Rate und den 5xx-Anteil hinzufügen: Fügen Sie diese zwei
   Argumente Traefiks Befehl hinzu (oder dieselben Schlüssel seiner
   statischen Konfiguration) und starten Sie es neu. Beide werden
   dann auf dem traefik-Entry-Point, Port 8080, bedient.

    ```
    --api.insecure=true
    --metrics.prometheus=true
    ```

2. Port 8080 antwortet ohne Passwort: Veröffentlichen Sie ihn nur auf
   einer für den DumbMonit-Host erreichbaren Adresse, filtern Sie
   ihn, sodass sonst nichts ihn erreicht, und prüfen Sie, dass er
   antwortet.

    ```
    curl -s http://traefik.lan:8080/api/overview
    ```

3. Um stattdessen ein Passwort zu verlangen, routen Sie /api und
   /metrics über einen eigenen Entry Point mit einer
   basicAuth-Middleware, wie die Dokumentation zeigt, und wählen Sie
   unten basicAuth user. Erstellen Sie den Benutzer dumbmonit mit
   htpasswd.

    ```
    htpasswd -nbB dumbmonit 'a-long-password'
    ```

4. Geben Sie in DumbMonit den Traefik-Host ein, zum Beispiel
   "traefik.lan", mit dem Port dieses Entry Points, falls er nicht
   8080 ist. Entfernen Sie das Häkchen bei Read the metrics, wenn
   /metrics anderswo bedient wird.

!!! warning
    Ein durch einen Konfigurationsfehler deaktivierter Router legt
    seine Sites lahm, und Traefik sagt das nur in seinem Log. Die
    Prüfung, ob ein Certificate-Resolver seine Zertifikate erhalten
    hat, braucht /api/certificates, das Traefik 3.7 bedient; ohne das
    kommt der Zertifikatsablauf nur aus den Metriken.

### Ein API-Entry-Point mit Passwort

Statt `--api.insecure=true`, geben Sie der API und den Metriken einen
eigenen Entry Point und stellen Sie beiden eine basicAuth-Middleware
vor. In der statischen Konfiguration:

```yaml
entryPoints:
  monitor:
    address: ":8082"
api: {}
metrics:
  prometheus:
    manualRouting: true
    addRoutersLabels: true
```

und in der dynamischen Konfiguration (File-Provider gezeigt;
Docker-Labels funktionieren genauso), mit der von `htpasswd`
ausgegebenen Zeile:

```yaml
http:
  middlewares:
    dumbmonit-auth:
      basicAuth:
        users:
          - "dumbmonit:$2y$05$…"
  routers:
    monitor-api:
      entryPoints: [monitor]
      rule: "PathPrefix(`/api`)"
      service: api@internal
      middlewares: [dumbmonit-auth]
    monitor-metrics:
      entryPoints: [monitor]
      rule: "Path(`/metrics`)"
      service: prometheus@internal
      middlewares: [dumbmonit-auth]
```

Geben Sie dann in DumbMonit `traefik.lan:8082` mit dem Benutzer
`dumbmonit` ein. `addRoutersLabels` ist, was die 5xx-Antworten pro
Router gibt; ohne es werden nur die Gesamtzahlen pro Entry Point
gezählt.

## Credentials

| Credential | Felder |
|---|---|
| Kein Passwort | Nichts: Der API-Port ist so gefiltert, dass nur DumbMonit ihn erreicht. |
| basicAuth-Benutzer | Benutzer und Passwort der basicAuth-Middleware vor `/api` und `/metrics`. |

Adresse: ein Hostname oder eine IP (`traefik.lan`), `host:port`, oder
eine vollständige URL. Optionen: Protokoll (standardmäßig HTTP),
API-Port (8080), Read the metrics (ein), Zertifikatsprüfung und
Anfrage-Timeout (10 s).

Ein Service ohne Healthcheck meldet jeden Server als up: Traefik
markiert einen Server nur als down, wenn ein `healthCheck`
fehlschlägt. Weighted- und Mirroring-Services haben keine eigenen
Server und werden nicht gezählt.

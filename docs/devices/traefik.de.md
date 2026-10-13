# Traefik {#traefik}

Der Reverse-Proxy: durch einen Konfigurationsfehler deaktivierte Router, Backend-Server, die
ihren Health-Check nicht bestehen, Zertifikate, die ein Resolver nie erhalten hat oder die nicht
erneuert werden, und der Anteil der 5xx-Antworten.

DumbMonit liest die API von Traefik, die nur liest: `/api/overview`, `/api/http/routers`,
`/api/http/services`, `/api/certificates` (von Traefik 3.7 bereitgestellt) und `/api/version`.
Werden die Prometheus-Metriken von Traefik auf demselben Port ausgeliefert, ergänzt `/metrics`
die Anfragerate und den Anteil der 5xx-Antworten. Keine Methode der API ändert etwas, und
DumbMonit sendet keine.

Die Fehler, auf die es ankommt, sind die stillen. Ein Router, dessen Service oder Middleware
nicht existiert oder dessen Regel sich nicht parsen lässt, ist deaktiviert: Seine Websites
antworten 404, und Traefik sagt das nur in seinem Log. Ein Service, dessen Server alle ihren
Health-Check nicht bestehen, antwortet 503 „no available server“. Ein Router, der von einem
ACME-Resolver ein Zertifikat verlangt, das dieser nicht erhalten kann, bekommt das
selbstsignierte Standardzertifikat von Traefik, das Browser ablehnen.

## Was überwacht wird {#what-it-watches}

Alle Metriken haben das Präfix `dumbmonit_traefik_`. Summen sind Zähler: Diagramme und Regeln
wandeln sie in Raten um.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version` |
| `routers`, `routers_errors`, `routers_warnings` | Router aller Protokolle und solche mit Fehler oder Warnung | |
| `services`, `services_errors`, `middlewares_errors` | dasselbe für Services und Middlewares | |
| `router_status` | pro HTTP-Router: 0 aktiviert, 1 Warnung, 2 deaktiviert | `router`, `provider` |
| `router_error_info` | Wert 1, für einen nicht aktivierten Router: der erste Fehler von Traefik | `router`, `error` |
| `service_error_info` | Wert 1, für einen nicht aktivierten Service: der erste Fehler von Traefik | `service`, `error` |
| `service_status` | pro Service: 0 aktiviert, 1 Warnung, 2 deaktiviert | `service` |
| `service_servers`, `service_servers_up` | Server eines lastverteilten Services und solche, die ihren Health-Check bestehen | `service` |
| `server_up` | 1, wenn der Server seinen Health-Check besteht (oder keinen hat) | `service`, `server` |
| `resolver_routers` | Router, die von einem Zertifikat-Resolver ihr Zertifikat verlangen | `resolver` |
| `resolver_routers_uncovered` | solche, deren Hostnamen noch kein von Traefik gehaltenes Zertifikat abdeckt (braucht `/api/certificates`) | `resolver` |
| `certificates`, `cert_expiry_days` | von Traefik gehaltene Zertifikate und die verbleibenden Tage bis zum Ablauf jedes einzelnen | `cert` |
| `requests_total`, `requests_5xx_total` | mit Metriken: Anfragen aller Entry Points und solche, die mit 5xx beantwortet wurden | |
| `router_requests_total`, `router_requests_5xx_total` | mit Metriken und Router-Labels: dasselbe pro Router | `router` |
| `config_reloads_total` | mit Metriken: Neuladungen der Konfiguration | |

Die eigenen Router und Services von Traefik (`@internal`) bleiben außen vor. Router, Services und
Server werden bis zu je 500 einzeln beschrieben; die Summen decken immer alles ab.

Ein Router, Service oder Server, den Traefik nicht mehr auflistet (der Docker-Provider entfernt
die Router eines gestoppten Containers sofort), wird einfach nicht mehr gemeldet: DumbMonit
behält dafür keinen Wert „gesund“, und seine Serien enden. Diagramme und Regeln zeigen seinen
letzten Wert möglicherweise noch einige Minuten, bis der Metrikspeicher die veraltete Serie
verwirft. Traefik antwortet für eine solche Route mit 404, was keine Regel meldet: Nur ein
aufgelisteter und deaktivierter Router tut das.

Die zutreffenden [integrierten Regeln](../alerting/rules.md#reverse-proxies-and-domains):
Traefik-Router deaktiviert, Traefik-Backend-Server ausgefallen, Traefik-Service ohne Server,
Traefik-Zertifikat nicht erhalten, Proxy-Zertifikat nicht erneuert, Reverse-Proxy antwortet mit
5xx, außerdem Gerät nicht erreichbar.

## Die Geräteseite {#the-device-page}

Das Panel über den Diagrammen liest, was die Prüfung gespeichert hat; das Öffnen der Seite
fragt Traefik nie ab. Es nennt in einem Satz, was nicht stimmt (deaktivierte Router und der
Grund von Traefik, Services ohne Server, Resolver, die nichts erhalten haben, bald ablaufende
Zertifikate, ein hoher Anteil an 5xx), dann die Router, Services, Anfragerate und den
5xx-Anteil, dann die aktiven Server pro Service, die Zertifikate, das am frühesten ablaufende
zuerst, und die 5xx-Antworten pro Router über 24 Stunden.

## DumbMonit die API von Traefik lesen lassen {#let-dumbmonit-read-traefiks-api}

1. Die API von Traefik liest nur: Router, Services und Zertifikate, nie eine Änderung. Aktivieren Sie sie zusammen mit den Prometheus-Metriken, die die Anfragerate und den Anteil der 5xx-Antworten liefern: Fügen Sie diese beiden Argumente zum Befehl von Traefik hinzu (oder dieselben Schlüssel zu seiner statischen Konfiguration) und starten Sie es neu. Beides wird dann auf dem Entry Point traefik, Port 8080, ausgeliefert.

    ```
    --api.insecure=true
    --metrics.prometheus=true
    ```

2. Port 8080 antwortet ohne Passwort: Veröffentlichen Sie ihn nur auf einer Adresse, die der DumbMonit-Host erreicht, filtern Sie ihn, sodass sonst niemand ihn erreicht, und prüfen Sie, dass er antwortet.

    ```
    curl -s http://traefik.lan:8080/api/overview
    ```

3. Um stattdessen ein Passwort zu verlangen, leiten Sie /api und /metrics über einen eigenen Entry Point mit einer basicAuth-Middleware, wie die Dokumentation zeigt, und wählen Sie unten basicAuth-Benutzer. Erstellen Sie den Benutzer dumbmonit mit htpasswd.

    ```
    htpasswd -nbB dumbmonit 'a-long-password'
    ```

4. Geben Sie in DumbMonit den Traefik-Host ein, zum Beispiel „traefik.lan“, mit dem Port dieses Entry Points, falls er nicht 8080 ist. Haken Sie Metriken lesen ab, wenn /metrics anderswo ausgeliefert wird.

!!! warning
    Ein durch einen Konfigurationsfehler deaktivierter Router legt seine Websites lahm, und Traefik sagt das nur in seinem Log. Die Prüfung, ob ein Zertifikat-Resolver seine Zertifikate erhalten hat, braucht /api/certificates, das Traefik 3.7 ausliefert; ohne sie stammt der Zertifikatsablauf nur aus den Metriken.

### Ein API-Entry-Point mit Passwort {#an-api-entry-point-with-a-password}

Statt `--api.insecure=true` geben Sie der API und den Metriken einen eigenen Entry Point und
setzen eine basicAuth-Middleware vor beide. In der statischen Konfiguration:

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

und in der dynamischen Konfiguration (File Provider gezeigt; Docker-Labels funktionieren
genauso), mit der von `htpasswd` ausgegebenen Zeile:

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

Geben Sie dann `traefik.lan:8082` in DumbMonit mit dem Benutzer `dumbmonit` ein.
`addRoutersLabels` liefert die 5xx-Antworten pro Router; ohne dieses werden nur die Summen pro
Entry Point gezählt.

## Zugangsdaten {#credentials}

| Zugangsdaten | Felder |
|---|---|
| Kein Passwort | Nichts: Der API-Port wird so gefiltert, dass nur DumbMonit ihn erreicht. |
| basicAuth-Benutzer | Benutzer und Passwort der basicAuth-Middleware vor `/api` und `/metrics`. |

Adresse: ein Hostname oder eine IP (`traefik.lan`), `host:port` oder eine vollständige URL.
Optionen: Protokoll (standardmäßig HTTP), API-Port (8080), Metriken lesen (an),
Zertifikatsprüfung und Anfrage-Timeout (10 s).

Ein Service ohne Health-Check meldet jeden Server als aktiv: Traefik markiert einen Server nur
dann als ausgefallen, wenn ein `healthCheck` fehlschlägt. Gewichtete und spiegelnde Services
haben keine eigenen Server und werden nicht gezählt.

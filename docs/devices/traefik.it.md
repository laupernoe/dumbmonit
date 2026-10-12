# Traefik

Il reverse proxy: router disattivati da un errore di configurazione, server
backend che falliscono il loro controllo di salute, certificati che un
resolver non ha mai ottenuto o che non sono rinnovati, e la quota di
risposte 5xx.

DumbMonit legge l'API di Traefik, che legge soltanto: `/api/overview`,
`/api/http/routers`, `/api/http/services`, `/api/certificates` (servito da
Traefik 3.7) e `/api/version`. Quando le metriche Prometheus di Traefik
sono servite sulla stessa porta, `/metrics` aggiunge il tasso di richieste
e la quota di risposte 5xx. Nessun metodo dell'API modifica nulla, e
DumbMonit non ne invia nessuno.

I guasti da tenere d'occhio sono quelli silenziosi. Un router il cui
servizio o middleware non esiste, oppure la cui regola non si analizza
correttamente, viene disattivato: i suoi siti rispondono 404, e Traefik lo
dice solo nel suo log. Un servizio i cui server falliscono tutti il loro
controllo di salute risponde 503 "no available server". Un router che
chiede a un resolver ACME un certificato che il resolver non può ottenere
riceve il certificato autofirmato predefinito di Traefik, che i browser
rifiutano.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_traefik_`. I totali sono
contatori: grafici e regole li trasformano in tassi.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version` |
| `routers`, `routers_errors`, `routers_warnings` | router di ogni protocollo, e quelli in errore o avviso | |
| `services`, `services_errors`, `middlewares_errors` | lo stesso per servizi e middleware | |
| `router_status` | per router HTTP: 0 abilitato, 1 avviso, 2 disattivato | `router`, `provider` |
| `router_error_info` | valore 1, per un router non abilitato: il primo errore di Traefik | `router`, `error` |
| `service_status` | per servizio: 0 abilitato, 1 avviso, 2 disattivato | `service` |
| `service_servers`, `service_servers_up` | server di un servizio con load balancing, e quelli che passano il loro controllo di salute | `service` |
| `server_up` | 1 quando il server passa il suo controllo di salute (o non ne ha) | `service`, `server` |
| `resolver_routers` | router che chiedono un certificato a un resolver di certificati | `resolver` |
| `resolver_routers_uncovered` | quelli i cui nomi host nessun certificato in possesso di Traefik copre ancora (richiede `/api/certificates`) | `resolver` |
| `certificates`, `cert_expiry_days` | certificati in possesso di Traefik, e i giorni rimanenti prima che ciascuno scada | `cert` |
| `requests_total`, `requests_5xx_total` | con le metriche: richieste di ogni entry point, e quelle risposte 5xx | |
| `router_requests_total`, `router_requests_5xx_total` | con metriche ed etichette router: lo stesso per router | `router` |
| `config_reloads_total` | con le metriche: ricariche di configurazione | |

I router e i servizi propri di Traefik (`@internal`) vengono esclusi.
Router, servizi e server sono descritti uno per uno fino a 500 ciascuno; i
totali coprono sempre tutto.

Le [regole integrate](../alerting/rules.md#reverse-proxies-and-domains) che
si applicano: Traefik router disabled, Traefik backend server down,
Traefik service without a server, Traefik certificate not obtained, Proxy
certificate not renewed, Reverse proxy answering 5xx, più Device
unreachable.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai Traefik. Indica cosa non va in una frase (router
disattivati e il motivo di Traefik, servizi senza un server, resolver che
non hanno ottenuto nulla, certificati vicini alla scadenza, un'alta quota
di 5xx), poi i router, i servizi, il tasso di richieste e la quota di 5xx,
poi i server attivi per servizio, i certificati più vicini alla scadenza
per primi, e le risposte 5xx per router sulle ultime 24 ore.

## Lasciare che DumbMonit legga l'API di Traefik

1. L'API di Traefik legge soltanto: router, servizi e certificati, mai una modifica. Abilitala con le metriche Prometheus, che aggiungono il tasso di richieste e la quota di risposte 5xx: aggiungi questi due argomenti al comando di Traefik (oppure le stesse chiavi alla sua configurazione statica) e riavvialo. Entrambe vengono allora servite sull'entry point traefik, porta 8080.

    ```
    --api.insecure=true
    --metrics.prometheus=true
    ```

2. La porta 8080 risponde senza password: pubblicala solo su un indirizzo che l'host DumbMonit raggiunge, filtrala così che nessun altro la raggiunga, e verifica che risponda.

    ```
    curl -s http://traefik.lan:8080/api/overview
    ```

3. Per richiedere invece una password, instrada /api e /metrics attraverso un entry point tuo con un middleware basicAuth, come mostra la documentazione, e scegli basicAuth user sotto. Crea l'utente dumbmonit con htpasswd.

    ```
    htpasswd -nbB dumbmonit 'a-long-password'
    ```

4. In DumbMonit, inserisci l'host Traefik, per esempio "traefik.lan", con la porta di quell'entry point se non è 8080. Deseleziona Read the metrics se /metrics è servito altrove.

!!! warning
    Un router disattivato da un errore di configurazione fa cadere i suoi siti, e Traefik lo dice solo nel suo log. Il controllo che un resolver di certificati abbia ottenuto i suoi certificati richiede /api/certificates, che Traefik 3.7 serve; senza di esso, la scadenza dei certificati proviene solo dalle metriche.

### Un entry point API con una password

Invece di `--api.insecure=true`, dai all'API e alle metriche un entry
point proprio e metti un middleware basicAuth davanti a entrambi. Nella
configurazione statica:

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

e nella configurazione dinamica (mostrato il file provider; le label
Docker funzionano nello stesso modo), con la riga stampata da `htpasswd`:

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

Poi inserisci `traefik.lan:8082` in DumbMonit con l'utente `dumbmonit`.
`addRoutersLabels` è ciò che fornisce le risposte 5xx per router; senza di
esso vengono contati solo i totali per entry point.

## Credenziali

| Credenziale | Campi |
|---|---|
| No password | Nessuna: la porta API è filtrata così che solo DumbMonit la raggiunga. |
| basicAuth user | L'utente e la password del middleware basicAuth davanti a `/api` e `/metrics`. |

Indirizzo: un nome host o IP (`traefik.lan`), `host:porta`, oppure un URL
completo. Opzioni: protocollo (HTTP per impostazione predefinita), porta
API (8080), Read the metrics (attivo), verifica del certificato e timeout
della richiesta (10 s).

Un servizio senza controllo di salute riporta ogni server come attivo:
Traefik segna un server come inattivo solo quando un `healthCheck`
fallisce. I servizi weighted e mirroring non hanno server propri e non
vengono contati.

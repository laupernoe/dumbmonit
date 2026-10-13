# Traefik

Il reverse proxy: router disabilitati da un errore di configurazione, server backend che non superano il controllo di salute, certificati che un resolver non ha mai ottenuto o che non vengono rinnovati, e la percentuale di risposte 5xx.

DumbMonit legge l'API di Traefik, che solo legge: `/api/overview`,
`/api/http/routers`, `/api/http/services`, `/api/certificates` (fornito da
Traefik 3.7) e `/api/version`. Quando le metriche Prometheus di Traefik vengono servite sulla
stessa porta, `/metrics` aggiunge la frequenza delle richieste e la percentuale di risposte 5xx. Nessun
metodo dell'API cambia nulla, e DumbMonit non invia alcun comando.

I guasti che vale la pena osservare sono quelli silenziosi. Un router il cui servizio o
middleware non esiste, o la cui regola non viene analizzata, è disabilitato: i suoi siti
rispondono 404 e Traefik lo dice solo nel suo log. Un servizio i cui server falliscono
tutti il loro controllo di salute risponde 503 "nessun server disponibile". Un router che chiede
a un resolver ACME un certificato che il resolver non riesce a ottenere ottiene il
certificato autofirmato predefinito di Traefik, che i browser rifiutano.

## Cosa monitora

Tutte le metriche hanno il prefisso `dumbmonit_traefik_`. I totali sono contatori: i grafici e
le regole li trasformano in frequenze.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version` |
| `routers`, `routers_errors`, `routers_warnings` | router di ogni protocollo, e quelli in errore o avviso | |
| `services`, `services_errors`, `middlewares_errors` | lo stesso per servizi e middleware | |
| `router_status` | per router HTTP: 0 abilitato, 1 avviso, 2 disabilitato | `router`, `provider` |
| `router_error_info` | valore 1, per un router non abilitato: il primo errore di Traefik | `router`, `error` |
| `service_error_info` | valore 1, per un servizio non abilitato: il primo errore di Traefik | `service`, `error` |
| `service_status` | per servizio: 0 abilitato, 1 avviso, 2 disabilitato | `service` |
| `service_servers`, `service_servers_up` | server di un servizio con bilanciamento del carico, e quelli che superano il controllo di salute | `service` |
| `server_up` | 1 quando il server supera il suo controllo di salute (o non ne ha uno) | `service`, `server` |
| `resolver_routers` | router che chiedono a un resolver di certificati il loro certificato | `resolver` |
| `resolver_routers_uncovered` | quelli i cui nomi host non sono ancora coperti da un certificato tenuto da Traefik (richiede `/api/certificates`) | `resolver` |
| `certificates`, `cert_expiry_days` | certificati che Traefik detiene, e i giorni rimasti prima che ognuno scada | `cert` |
| `requests_total`, `requests_5xx_total` | con metriche: richieste di ogni entry point, e quelle risposte 5xx | |
| `router_requests_total`, `router_requests_5xx_total` | con metriche ed etichette del router: lo stesso per router | `router` |
| `config_reloads_total` | con metriche: ricaricamenti della configurazione | |

I router e i servizi interni di Traefik (`@internal`) sono esclusi. Router,
servizi e server sono descritti uno per uno fino a 500 ciascuno; i totali
coprono sempre tutto.

Un router, servizio o server che Traefik non elenca più (il provider Docker
abbandona i router di un contenitore interrotto immediatamente) non viene più segnalato:
DumbMonit non mantiene un valore "healthy" per esso e le sue serie terminano. I grafici
e le regole possono ancora mostrare il suo ultimo valore per alcuni minuti, fino a quando l'archivio di metriche
non abbandona la serie obsoleta. Traefik risponde 404 per tale percorso, che nessuna
regola segnala: solo un router che è elencato e disabilitato lo fa.

Le [regole integrate](../alerting/rules.md#reverse-proxies-and-domains) che
si applicano: router Traefik disabilitato, server backend Traefik inattivo, servizio Traefik
senza server, certificato Traefik non ottenuto, certificato proxy non
rinnovato, reverse proxy che risponde 5xx, più dispositivo irraggiungibile.

## La pagina del dispositivo

Il pannello sopra i grafici legge quello che la sonda ha memorizzato; aprire la pagina non
interroga mai Traefik. Dice cosa è sbagliato in una frase (router disabilitati e
motivo di Traefik, servizi senza server, resolver che non hanno ottenuto nulla,
certificati prossimi alla scadenza, una percentuale elevata di 5xx), quindi i router,
servizi, frequenza delle richieste e percentuale di 5xx, quindi i server attivi per servizio, i
certificati che scadono prima per primi, e le risposte 5xx per router negli ultimi 24
ore.

## Consenti a DumbMonit di leggere l'API di Traefik

1. L'API di Traefik legge solo: router, servizi e certificati, mai una modifica. Abilitala con le metriche Prometheus, che aggiungono la frequenza delle richieste e la percentuale di risposte 5xx: aggiungi questi due argomenti al comando di Traefik (o le stesse chiavi alla sua configurazione statica) e riavvialo. Entrambi vengono quindi serviti sull'entry point di traefik, porta 8080.

    ```
    --api.insecure=true
    --metrics.prometheus=true
    ```

2. La porta 8080 risponde senza password: pubblicala solo su un indirizzo che l'host DumbMonit raggiunge, filtralo in modo che nient'altro lo faccia, e verifica che risponda.

    ```
    curl -s http://traefik.lan:8080/api/overview
    ```

3. Per richiedere invece una password, instrada /api e /metrics attraverso un entry point che hai creato con un middleware basicAuth, come mostra la documentazione, e scegli l'utente basicAuth di seguito. Crea l'utente dumbmonit con htpasswd.

    ```
    htpasswd -nbB dumbmonit 'a-long-password'
    ```

4. In DumbMonit, inserisci l'host di Traefik, ad esempio "traefik.lan", con la porta di quell'entry point se non è 8080. Deseleziona Leggi le metriche se /metrics viene servito altrove.

!!! warning
    Un router disabilitato da un errore di configurazione mette offline i suoi siti e Traefik lo dice solo nel suo log. Il controllo che un resolver di certificati ha ottenuto i suoi certificati richiede /api/certificates, che Traefik 3.7 serve; senza di esso, la scadenza del certificato viene solo dalle metriche.

### Un entry point API con password

Invece di `--api.insecure=true`, dai all'API e alle metriche un entry point
proprio e metti un middleware basicAuth davanti a entrambi. Nella configurazione
statica:

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

e nella configurazione dinamica (provider file mostrato; le etichette Docker funzionano
allo stesso modo), con la riga stampata da `htpasswd`:

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
`addRoutersLabels` è ciò che dà le risposte 5xx per router; senza di essa solo
i totali per entry point vengono conteggiati.

## Credenziali

| Credenziale | Campi |
|---|---|
| Nessuna password | Niente: la porta API è filtrata in modo che solo DumbMonit la raggiunga. |
| Utente basicAuth | L'utente e la password del middleware basicAuth davanti a `/api` e `/metrics`. |

Indirizzo: un nome host o IP (`traefik.lan`), `host:port`, o un URL completo.
Opzioni: protocollo (HTTP per impostazione predefinita), porta API (8080), Leggi le metriche (attivo),
verifica certificato e timeout delle richieste (10 s).

Un servizio senza controllo di salute segnala ogni server come attivo: Traefik contrassegna
un server come inattivo solo quando un `healthCheck` fallisce. I servizi con ponderazione e mirroring non
hanno server propri e non vengono conteggiati.

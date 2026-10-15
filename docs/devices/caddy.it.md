# Caddy

Il server web e reverse proxy: upstream che falliscono i loro controlli di
salute, una ricarica di configurazione fallita, errori dei handler e la
quota di risposte 5xx.

DumbMonit legge tre cose dall'API di amministrazione di Caddy, tutte con
`GET`: `/config/apps/http/servers` (la configurazione HTTP che Caddy
esegue, mai la sezione `tls` dove vivono i token dei provider DNS),
`/reverse_proxy/upstreams` (i backend e i fallimenti che il controllo di
salute passivo ricorda) e `/metrics` (stato di salute upstream, ricariche,
richieste ed errori).

L'API di amministrazione di Caddy non è di sola lettura: chiunque la
raggiunga può caricare una nuova configurazione o fermare Caddy. Per
questo ascolta su `localhost:2019`. La configurazione sotto la mantiene lì
e lascia che Caddy stesso pubblichi una vista di sola lettura di essa,
protetta da una password, che risponde `GET` sui tre percorsi sopra e
rifiuta tutto il resto.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_caddy_`. I totali sono
contatori: grafici e regole li trasformano in tassi.

| Metrica | Cosa | Etichette |
|---|---|---|
| `config_loaded` | 1 quando Caddy esegue almeno un server HTTP | |
| `servers`, `routes` | server HTTP e route nella configurazione | |
| `config_last_reload_ok` | 0 quando l'ultima ricarica di configurazione è fallita | |
| `config_last_reload_timestamp_seconds` | orario dell'ultima ricarica riuscita | |
| `upstreams`, `upstreams_unhealthy` | backend di ogni `reverse_proxy`, e quelli in errore | |
| `upstream_healthy` | 1 quando né il controllo di salute attivo né quello passivo escludono il backend | `upstream` |
| `upstream_fails` | richieste fallite che il controllo di salute passivo ricorda attualmente | `upstream` |
| `upstream_requests` | richieste in corso verso il backend | `upstream` |
| `requests_total`, `requests_5xx_total`, `request_errors_total` | con l'opzione `metrics`: richieste, risposte 5xx ed errori dei handler | |
| `server_requests_total`, `server_requests_5xx_total`, `server_request_errors_total` | lo stesso per server (`srv0`, `srv1`… oppure i nomi che hai dato) | `server` |

`upstream_healthy` richiede `/metrics`: il controllo di salute attivo è
visibile solo lì. Una vista di sola lettura che non lascia passare
`/metrics` riporta comunque i backend e i loro fallimenti ricordati. Caddy
non pubblica la sua versione tramite l'API di amministrazione.

Le [regole integrate](../alerting/rules.md#reverse-proxies-and-domains) che
si applicano: Caddy upstream unhealthy, Caddy configuration reload failed,
Reverse proxy answering 5xx, più Monitor unreachable.

## La pagina del monitor

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai Caddy. Indica cosa non va in una frase (nessuna
configurazione, una ricarica fallita, upstream in errore, un'alta quota di
5xx), poi i server, le route, gli upstream, il tasso di richieste, la quota
di 5xx e gli errori dei handler sulle ultime 24 ore, poi ogni upstream con
i suoi fallimenti ricordati.

## Pubblicare una vista di sola lettura dell'API di amministrazione di Caddy

1. L'API di amministrazione di Caddy può caricare una nuova configurazione e fermare Caddy: deve restare fuori portata di tutto tranne DumbMonit. Lasciala dove Caddy la mette, su localhost:2019, e lascia che Caddy pubblichi una vista di sola lettura di essa: solo GET, sui tre percorsi che DumbMonit legge, dietro una password.

2. Crea l'hash della password per l'utente dumbmonit.

    ```
    caddy hash-password --plaintext 'a-long-password'
    ```

3. Aggiungi questo sito al Caddyfile, con quell'hash al posto di HASH, poi ricarica Caddy. Risponde sulla porta 2020 e rifiuta tutto tranne quelle tre letture.

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

4. Opzionale: aggiungi metrics al blocco di opzioni globali in cima al Caddyfile per contare richieste, errori e risposte 5xx per server. Lo stato di salute upstream e le ricariche vengono riportati anche senza di esso.

    ```
    metrics
    ```

5. Filtra la porta 2020 così che solo l'host DumbMonit la raggiunga. In DumbMonit, inserisci l'host Caddy con quella porta, per esempio "caddy.lan:2020", e l'utente dumbmonit con la sua password.

!!! warning
    Non pubblicare mai l'API di amministrazione stessa sulla rete senza un firewall davanti: chiunque raggiunga la porta 2019 può sostituire la configurazione o fermare Caddy, e l'opzione origins non ferma un client che non invia alcun header Origin. Se la esponi su una rete privata, filtra la porta così che solo l'host DumbMonit la raggiunga, e scegli No password.

Il Caddyfile mantiene il suo significato senza indentazione; `caddy fmt` lo
indenta. `header_up Host localhost:2019` è necessario perché l'API di
amministrazione risponde solo alle richieste indirizzate all'indirizzo su
cui ascolta. Con l'immagine container Caddy, esegui il comando hash con
`docker exec`.

Una configurazione caricata come JSON invece che da un Caddyfile riceve lo
stesso sito come un server extra con un handler `basic_auth`, un matcher
`method` e `path` e un `reverse_proxy` verso `localhost:2019`.

## Credenziali

| Credenziale | Campi |
|---|---|
| Read-only view | L'utente e la password del blocco `basic_auth` sopra (consigliato). |
| No password | Nessuna: l'API di amministrazione stessa, filtrata così che solo DumbMonit la raggiunga. |

Indirizzo: un nome host o IP con la porta della vista di sola lettura
(`caddy.lan:2020`), oppure un URL completo. Opzioni: protocollo (HTTP per
impostazione predefinita), porta (2019, l'API di amministrazione stessa),
verifica del certificato e timeout della richiesta (10 s).

Il controllo di salute passivo (`fail_duration`) ricorda i fallimenti solo
per quella durata: un backend che fallisce una volta e si riprende viene
riportato in errore finché non passa quel tempo. La regola attende cinque
minuti prima di attivarsi.

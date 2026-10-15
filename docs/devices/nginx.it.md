# Nginx

Il server web e reverse proxy: connessioni attive e in attesa, i tassi
accept/handled/request, e lo stato di salute degli upstream quando l'API
JSON commerciale NGINX Plus è disponibile.

DumbMonit legge il modulo `stub_status` integrato di Nginx, una manciata di
contatori in testo semplice, senza API e senza alcun account necessario su
una rete correttamente filtrata. Quando NGINX Plus commerciale è installato,
DumbMonit può anche leggere la sua API JSON per lo stato di salute degli
upstream e i contatori per zona.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_nginx_`. Le metriche `_total`
sono contatori: grafici e regole le trasformano in tassi.

| Metrica | Cosa | Etichette |
|---|---|---|
| `connections_active` | connessioni attualmente aperte | |
| `connections_reading`, `connections_writing`, `connections_waiting` | connessioni per cosa stanno facendo | |
| `accepts_total`, `handled_total` | connessioni accettate, ed effettivamente gestite | |
| `requests_total` | richieste servite dall'avvio di Nginx | |
| `version_info` | presenza; l'header di risposta `Server`, quando Nginx lo invia | `server` |
| `plus_upstream_server_up` | 1 quando NGINX Plus segna il server come up | `upstream`, `server` |
| `plus_upstream_server_active`, `plus_upstream_server_requests_total`, `plus_upstream_server_fails_total`, `plus_upstream_server_5xx_total` | contatori NGINX Plus per server | `upstream`, `server` |
| `plus_zone_requests_total`, `plus_zone_5xx_total`, `plus_zone_received_bytes_total`, `plus_zone_sent_bytes_total` | contatori NGINX Plus per zona | `zone` |

Una differenza tra `accepts_total` e `handled_total` significa connessioni
che Nginx ha scartato prima di poterle servire (di solito `worker_connections`
raggiunto). Le metriche `plus_*` appaiono solo quando "Read the NGINX Plus
API" è spuntata e NGINX Plus risponde; la build open source non ha un
"upstream" da riportare con il solo `stub_status`.

## La pagina del monitor

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai Nginx. Mostra connessioni attive, in lettura, in
scrittura e in attesa, i tassi accept/handled/request sulle ultime 24 ore,
e, con NGINX Plus, lo stato di salute degli upstream e il traffico per zona.

## Abilitare il modulo stub_status di Nginx

1. Nginx include stub_status integrato: richiede solo una location che lo attivi. Aggiungi questo al server block che Nginx già serve (oppure a un suo proprio server block su una porta propria), poi ricarica Nginx.

    ```
    location /basic_status {
    stub_status;
    }
    ```

2. Limita quella location all'host DumbMonit, così nessun altro può leggerla; adatta l'indirizzo di conseguenza.

    ```
    location /basic_status {
    stub_status;
    allow 10.0.0.5;
    deny all;
    }
    nginx -s reload
    ```

3. Verifica che risponda prima di configurare DumbMonit.

    ```
    curl -s http://nginx.lan/basic_status
    ```

4. In DumbMonit, inserisci l'host Nginx, per esempio "nginx.lan", e il percorso se non è "/basic_status". Se NGINX Plus è installato, spunta "Read the NGINX Plus API" per aggiungere lo stato di salute degli upstream e i contatori per zona.

!!! warning
    stub_status conta solo connessioni e richieste: un errore di
    configurazione in un server block, oppure un backend che nginx non può
    raggiungere, non appare qui. La build open source non ha alcuna nozione
    di "upstream": esiste solo in NGINX Plus, letto tramite la sua propria
    API quando abilitata.

L'esempio sopra limita la location per IP, la configurazione più semplice
per un homelab; un blocco `basic_auth` funziona altrettanto bene se la
pagina di stato non può stare dietro una propria regola di firewall.
`stub_status` stesso non può essere protetto da password direttamente da
Nginx — la password, se presente, appartiene al blocco `location`
circostante.

## Credenziali

| Credenziale | Campi |
|---|---|
| No password | Nessuna: la pagina di stato è filtrata così che solo DumbMonit la raggiunga. |
| Basic auth | L'utente e la password del blocco che protegge la pagina di stato, se presente. |

Indirizzo: un nome host o IP, con la porta se non è 80. Opzioni: protocollo
(HTTP per impostazione predefinita), porta, percorso di stato
(`/basic_status` per impostazione predefinita), verifica del certificato,
timeout della richiesta (10 s), e l'interruttore e la versione dell'API
NGINX Plus.

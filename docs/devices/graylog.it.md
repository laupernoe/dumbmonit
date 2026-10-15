# Graylog

Il server di log e il cluster di ricerca dietro di esso: se Graylog elabora
i messaggi, quanti attendono nel suo journal, quanto sono pieni i suoi
buffer, cosa entra e cosa esce, lo stato di salute di OpenSearch o
Elasticsearch come Graylog lo vede, gli input che non sono riusciti ad
avviarsi, e i messaggi che non sono riusciti a essere indicizzati.

DumbMonit legge l'API REST di Graylog con un utente dedicato che possiede
il ruolo integrato **Reader**. Non cerca mai messaggi, non avvia o ferma
mai un input e non conferma mai una notifica.

Il guasto da tenere d'occhio: **il cluster di ricerca che smette di stare al
passo**. Graylog continua ad accettare messaggi e li scrive nel suo journal
su disco; l'interfaccia web si apre, le ricerche funzionano, e semplicemente
non mostrano nulla di recente. Il journal cresce finché non è pieno, poi i
messaggi vengono scartati. L'arretrato del journal, lo stato di salute del
cluster di ricerca e i fallimenti di indicizzazione lo dicono molto prima.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_graylog_`. Il journal, i
buffer e il throughput sono quelli **del nodo con cui DumbMonit parla**: in
un cluster Graylog, aggiungi ogni nodo come proprio monitor.

| Metrica | Cosa | Etichette |
|---|---|---|
| `processing` | 1 mentre il nodo elabora messaggi, 0 quando l'elaborazione è in pausa | |
| `running`, `lb_alive` | il ciclo di vita del nodo è `running`; si segnala vivo ai load balancer | |
| `version_info` | valore 1 | `version` |
| `journal_uncommitted_entries` | messaggi scritti nel journal e non ancora elaborati | |
| `journal_size_bytes`, `journal_size_limit_bytes`, `journal_used_percent` | | |
| `journal_append_per_second`, `journal_read_per_second` | | |
| `buffer_used_percent` | quanto sono pieni i buffer di input, process e output | `buffer` |
| `messages_in_total`, `messages_out_total` | messaggi ricevuti, e scritti verso il cluster di ricerca | |
| `indexer_status` | stato di salute del cluster di ricerca: 0 green, 1 yellow, 2 red, 3 irraggiungibile da Graylog | |
| `indexer_shards` | shard per stato: `active`, `initializing`, `relocating`, `unassigned` | `state` |
| `inputs_running`, `inputs_failed` | input su questo nodo | |
| `input_failed` | valore 1 per input non riuscito ad avviarsi | `input` (il suo titolo) |
| `output_failures_total`, `processing_failures_total`, `invalid_timestamps_total` | scritture verso il cluster di ricerca fallite, messaggi falliti nell'elaborazione o indicizzazione, messaggi con un timestamp non valido | |
| `notifications`, `notifications_urgent`, `notification` | notifiche proprie di Graylog (permesso opzionale, vedi sotto) | `type`, `severity` |
| `cluster_nodes` | nodi nel cluster Graylog | |
| `scrape_errors` | chiamate fallite nell'ultima sonda | |

Il cluster di ricerca viene letto tramite Graylog, quindi un monitor
OpenSearch o Elasticsearch separato non è necessario per sapere se è verde;
quando Graylog non può raggiungerlo affatto, `indexer_status` legge 3.

Le [regole integrate](../alerting/rules.md#log-and-metrics-servers) che si
applicano: Graylog not processing, Graylog search cluster down, Graylog
search cluster yellow, Graylog journal filling, Graylog input failed,
Graylog indexing failures, più Monitor unreachable.

## La pagina del monitor

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai Graylog. Indica cosa non va in una frase
(elaborazione, cluster di ricerca, arretrato del journal, input falliti per
nome, fallimenti di indicizzazione, notifiche urgenti), poi i messaggi in
entrata e in uscita al secondo, l'arretrato del journal e il numero di
nodi, poi i buffer, gli shard per stato e le notifiche proprie di Graylog.

## Creare un utente e un token di sola lettura in Graylog

1. In Graylog: System → Users and Teams → Create user. Dai all'utente il nome seguente, una password lunga e casuale, e assegna solo il ruolo Reader. Reader può leggere lo stato del nodo, il journal, i buffer, il throughput, gli input e lo stato di salute del cluster di ricerca, e nessun messaggio affatto finché nessuno stream è condiviso con lui.

    ```
    dumbmonit
    ```

2. Apri la pagina Edit tokens di quell'utente, crea un token con il nome DumbMonit e copialo subito: Graylog lo mostra solo una volta.

3. Opzionale: le notifiche proprie di Graylog (un input non riuscito ad avviarsi, le soglie del disco, un journal quasi pieno) richiedono un permesso in più, notifications:read, che nessun ruolo integrato concede da solo. Con un account autorizzato a gestire i ruoli, crea un ruolo che possiede solo quel permesso e dallo all'utente dumbmonit. Senza di esso, le notifiche vengono saltate e nulla altro cambia.

    ```
    curl -u YOUR_USER -H 'X-Requested-By: cli' -H 'Content-Type: application/json' -X POST https://graylog.lan/api/roles -d '{"name":"DumbMonit Notifications","description":"Read system notifications","permissions":["notifications:read"],"read_only":false}'
    curl -u YOUR_USER -H 'X-Requested-By: cli' -X PUT 'https://graylog.lan/api/roles/DumbMonit%20Notifications/members/dumbmonit'
    ```

4. In DumbMonit, inserisci l'indirizzo del nodo Graylog, per esempio "graylog.lan" (porta 9000) oppure "https://graylog.lan" dietro un reverse proxy, e incolla il token. In un cluster Graylog, aggiungi ogni nodo: journal, buffer e throughput sono per nodo.

5. DumbMonit legge soltanto. Non cerca mai messaggi, non avvia o ferma mai un input e non conferma mai una notifica.

!!! warning
    Non riutilizzare l'account con cui accedi: un token compromesso porta ogni diritto del suo utente, mentre il ruolo Reader sopra non può modificare nulla. Graylog serve HTTP semplice a meno che tu non abbia configurato TLS, e il token viaggia con ogni richiesta: su una rete non affidabile, metti Graylog dietro HTTPS.

Cosa copre ciascun permesso di Reader: `system:read` lo stato del nodo e i
nodi del cluster, `journal:read` il journal, `metrics:read` e
`throughput:read` il throughput e i buffer, `inputs:read` gli input,
`indexercluster:read` lo stato di salute del cluster di ricerca. Il ruolo
Reader possiede anche `messages:read`, che gli permette di leggere messaggi
solo dagli stream esplicitamente condivisi con lui: non condividerne
nessuno. I dettagli dei fallimenti di indicizzazione (`indices:failures`)
non sono necessari: DumbMonit conta i fallimenti dalle metriche proprie di
Graylog.

## Credenziali

| Credenziale | Campi |
|---|---|
| Access token (consigliato) | Un token dell'utente dedicato. Graylog lo attende come autenticazione HTTP di base con il token come nome utente e la parola `token` come password; DumbMonit lo fa per te. |
| User name / password | L'utente dedicato stesso. |

Indirizzo: un nome host o IP (`graylog.lan`), `host:porta`, oppure un URL
completo (`https://graylog.lan`); un `/api` finale viene accettato.
Opzioni: protocollo (HTTP per impostazione predefinita), porta (9000),
verifica del certificato e timeout della richiesta (10 s).

# Redis e Valkey

Lo store in memoria stesso: memoria rispetto a `maxmemory`, connessioni
rifiutate, il collegamento di replicazione e il ritardo di ogni replica,
snapshot falliti e scritture dell'append-only file, e la dimensione dello
keyspace.

DumbMonit apre una connessione, invia `AUTH`, poi `INFO`, e la chiude.
L'account che usa può eseguire `INFO` e `PING` e nient'altro: non può
leggere, scrivere o nemmeno elencare una chiave. Redis 6 e successivi,
Valkey e i fork che rispondono a `INFO` nello stesso modo (KeyDB,
Dragonfly) sono supportati.

I guasti da tenere d'occhio sono quelli silenziosi. A `maxmemory`, Redis o
evacua chiavi senza avvisare nessuno oppure rifiuta ogni scrittura con
`OOM command not allowed`, a seconda di `maxmemory-policy`; senza
`maxmemory`, cresce finché il kernel non lo uccide. Uno snapshot fallito fa
sì che Redis rifiuti le scritture finché `stop-writes-on-bgsave-error` è
attivo, cosa che è per impostazione predefinita. Una replica il cui
collegamento con il suo primario è inattivo continua a rispondere alle
letture, con dati obsoleti.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_redis_`. I totali sono
contatori: grafici e regole li trasformano in tassi.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `product` (`redis`, `valkey`), `version`, `mode` |
| `role_info` | valore 1 | `role`: `master` o `replica` |
| `memory_used_bytes`, `memory_rss_bytes`, `memory_peak_bytes`, `memory_fragmentation_ratio` | memoria come la conta Redis, e come la conta il sistema | |
| `memory_max_bytes`, `memory_used_percent` | `maxmemory` e la quota usata; assente quando `maxmemory` è 0 (nessun limite) | |
| `maxmemory_policy_info` | valore 1 | `policy` |
| `connected_clients`, `blocked_clients`, `max_clients` | | |
| `rejected_connections_total` | connessioni rifiutate perché `maxclients` è stato raggiunto | |
| `evicted_keys_total`, `expired_keys_total` | chiavi evacuate a `maxmemory`, chiavi scadute | |
| `keyspace_hits_total`, `keyspace_misses_total` | ricerche che hanno trovato la loro chiave, e quelle che non l'hanno trovata | |
| `commands_processed_total`, `connections_received_total`, `ops_per_second` | | |
| `error_replies_total`, `acl_denied_auth_total` | risposte di errore inviate, e login rifiutati (una password che viene indovinata si vede qui) | |
| `keys`, `keys_expiring` | chiavi in tutti i database, e quelle con una scadenza; sempre presente, anche a zero | |
| `db_keys` | chiavi per database | `db` |
| `rdb_last_save_ok` | 0 quando l'ultimo snapshot (`BGSAVE`) è fallito | |
| `rdb_changes_since_last_save`, `rdb_last_save_timestamp_seconds` | | |
| `aof_enabled`, `aof_last_write_ok`, `aof_last_rewrite_ok` | append-only file; i due stati solo quando è attivo | |
| `loading` | 1 mentre Redis carica il suo dataset all'avvio | |
| `connected_replicas` | replica collegate a questo primario | |
| `replica_online`, `replica_lag_seconds`, `replica_lag_bytes` | visto dal primario, per replica: online o no, secondi dall'ultima conferma, byte di ritardo | `replica` (`ip:porta`) |
| `master_link_up`, `master_last_io_seconds`, `master_link_down_seconds`, `master_sync_in_progress` | su una replica: il suo collegamento con il primario | |
| `uptime_seconds`, `cluster_enabled` | | |

Le [regole integrate](../alerting/rules.md#databases-message-broker-and-security-engine)
che si applicano: Redis memory near maxmemory, Redis refusing connections,
Redis replication link down, Redis replica lagging, Redis snapshot failed,
Redis append-only file write failed, più Device unreachable.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai Redis. Indica cosa non va in una frase
(replicazione, memoria rispetto a `maxmemory` e la policy che si applica al
limite, persistenza, connessioni rifiutate), poi le chiavi, la memoria, i
client, i comandi al secondo, il tasso di successo, le chiavi evacuate
nelle ultime 24 ore e l'uptime, poi le chiavi per database e il ritardo di
ogni replica.

## Creare un utente Redis che possa solo eseguire INFO

1. Su Redis 6 o successivo, oppure Valkey, crea un utente che possa eseguire INFO e PING e nient'altro, senza accesso ad alcuna chiave. In redis-cli, autenticato con un utente autorizzato a gestire le ACL:

    ```
    ACL SETUSER dumbmonit on >a-long-password -@all +info +ping
    ```

2. Rendilo permanente: ACL SAVE se il server carica i suoi utenti da un aclfile, CONFIG REWRITE altrimenti.

    ```
    ACL SAVE
    ```

3. Su Redis 5 o più vecchio, oppure con solo requirepass impostato, scegli Password only più sotto. Quella password porta ogni diritto: preferisci un utente ACL ogni volta che il server lo supporta.

4. In DumbMonit, inserisci l'indirizzo del server, per esempio "cache.lan" oppure "cache.lan:6380". Se il server accetta solo TLS su quella porta (tls-port), spunta TLS più sotto.

5. DumbMonit invia AUTH, poi INFO. Non legge, scrive o elenca mai una chiave.

!!! warning
    Redis non dà alcun avviso prima di maxmemory: al limite evacua chiavi silenziosamente oppure rifiuta ogni scrittura, a seconda di maxmemory-policy. Senza maxmemory cresce finché il kernel non lo uccide. DumbMonit avvisa sopra il 90% di maxmemory.

## Credenziali

| Credenziale | Campi |
|---|---|
| ACL user (consigliato) | L'utente creato sopra e la sua password, inviati come `AUTH user password`. |
| Password only | Il valore `requirepass`, inviato come `AUTH password`. |
| No authentication | Per un server senza alcuna password. |

Indirizzo: un nome host o IP (`cache.lan`), `host:porta`, oppure una
stringa di connessione (`redis://cache.lan:6380`; tutto ciò che segue la
porta viene ignorato, e le credenziali scritte in essa non vengono mai
usate). Opzioni: porta (6379), TLS (disattivato), verifica del
certificato, e timeout per l'intero controllo (10 s). Con TLS attivo, il
certificato viene verificato contro le autorità pubbliche integrate in
DumbMonit; per un'autorità privata, spunta Accept an unverifiable
certificate.

In un deployment Redis Sentinel o Cluster, aggiungi ogni server che vuoi
osservare: ciascuno riporta la propria memoria, i client e la persistenza,
e un primario riporta le sue replica. Sentinel stesso non è supportato.

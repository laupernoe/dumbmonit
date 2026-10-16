# Redis e Valkey

Il negozio in memoria stesso: la memoria rispetto a `maxmemory`, le connessioni
rifiutate, il link di replica e il lag di ogni replica, gli snapshot non riusciti
e le scritture nel file di sola aggiunta, e la dimensione dello spazio chiavi.

DumbMonit apre una connessione, invia `AUTH`, quindi `INFO`, e la chiude. L'account
che usa può eseguire `INFO` e `PING` e nient'altro: non può leggere, scrivere o
nemmeno elencare una chiave. Redis 6 e successivi, Valkey e i fork che rispondono
a `INFO` allo stesso modo (KeyDB, Dragonfly) sono coperti.

I fallimenti che vale la pena osservare sono quelli silenziosi. A `maxmemory`, Redis
o evita le chiavi senza dirlo a nessuno o rifiuta ogni scrittura con `OOM command
not allowed`, a seconda di `maxmemory-policy`; senza `maxmemory`, cresce finché il
kernel non lo uccide. Uno snapshot non riuscito fa sì che Redis rifiuti le scritture
finché `stop-writes-on-bgsave-error` è acceso, il che è per impostazione predefinita.
Una replica il cui link al suo primary è giù continua a rispondere alle letture,
con dati stantii.

## Cosa monitora

Tutte le metriche hanno il prefisso `dumbmonit_redis_`. I totali sono contatori:
i grafici e le regole li trasformano in rate.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `product` (`redis`, `valkey`), `version`, `mode` |
| `role_info` | valore 1 | `role`: `master` o `replica` |
| `memory_used_bytes`, `memory_rss_bytes`, `memory_peak_bytes`, `memory_fragmentation_ratio` | memoria come la conta Redis, e come la conta il sistema | |
| `memory_max_bytes`, `memory_used_percent` | `maxmemory` e la quota usata; assente quando `maxmemory` è 0 (nessun limite) | |
| `maxmemory_policy_info` | valore 1 | `policy` |
| `connected_clients`, `blocked_clients`, `max_clients` | | |
| `rejected_connections_total` | connessioni rifiutate perché è stato raggiunto `maxclients` | |
| `evicted_keys_total`, `expired_keys_total` | chiavi evitate a `maxmemory`, chiavi scadute | |
| `keyspace_hits_total`, `keyspace_misses_total` | ricerche che hanno trovato la loro chiave, e quelle che non lo hanno fatto | |
| `commands_processed_total`, `connections_received_total`, `ops_per_second` | | |
| `error_replies_total`, `acl_denied_auth_total` | risposte di errore inviate, e accessi rifiutati (un tentativo di indovinare una password mostra qui) | |
| `keys`, `keys_expiring` | chiavi in tutti i database, e quelle con una scadenza; sempre presenti, anche a zero | |
| `db_keys` | chiavi per database | `db` |
| `rdb_last_save_ok` | 0 quando l'ultimo snapshot (`BGSAVE`) è fallito | |
| `rdb_changes_since_last_save`, `rdb_last_save_timestamp_seconds` | | |
| `aof_enabled`, `aof_last_write_ok`, `aof_last_rewrite_ok` | file di sola aggiunta; i due status solo quando è acceso | |
| `loading` | 1 mentre Redis carica il suo dataset all'avvio | |
| `connected_replicas` | repliche collegate a questo primary | |
| `replica_online`, `replica_lag_seconds`, `replica_lag_bytes` | visto dal primary, per replica: online o non, secondi da quando ha riconosciuto l'ultima volta, byte indietro | `replica` (`ip:port`) |
| `master_link_up`, `master_last_io_seconds`, `master_link_down_seconds`, `master_sync_in_progress` | su una replica: il suo link al primary | |
| `uptime_seconds`, `cluster_enabled` | | |

Le [regole incorporate](../alerting/rules.md#databases-message-broker-and-security-engine)
che si applicano: memoria Redis vicino a maxmemory, Redis rifiuta le connessioni,
link di replica Redis giù, replica Redis in ritardo, snapshot Redis non riuscito,
scrittura file di sola aggiunta Redis non riuscita, più Device irraggiungibile.

## La pagina del monitor

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la pagina
non interroga mai Redis. Dice cosa c'è che non va in una frase (replica, memoria
rispetto a `maxmemory` e la politica che si applica al limite, persistenza, connessioni
rifiutate), poi le chiavi, la memoria, i client, i comandi al secondo, il rapporto hit,
le chiavi evitate nelle ultime 24 ore e l'uptime, poi le chiavi per database e il lag
di ogni replica.

## Crea un utente Redis che può solo eseguire INFO

1. Su Redis 6 o successivo, o Valkey, crea un utente che può eseguire INFO e PING e nient'altro, senza accesso a nessuna chiave. In redis-cli, acceduto con un utente autorizzato a gestire gli ACL:

    ```
    ACL SETUSER dumbmonit on >a-long-password -@all +info +ping
    ```

2. Rendilo permanente: ACL SAVE se il server carica i suoi utenti da un aclfile, CONFIG REWRITE altrimenti.

    ```
    ACL SAVE
    ```

3. Su Redis 5 o precedente, o con solo requirepass impostato, scegli Password only qui sotto. Quella password comporta ogni diritto: preferisci un utente ACL quando il server lo supporta.

4. In DumbMonit, inserisci l'indirizzo del server, ad esempio "cache.lan" o "cache.lan:6380". Se il server accetta solo TLS su quella porta (tls-port), abilita TLS qui sotto.

5. DumbMonit invia AUTH, quindi INFO. Non legge, scrive o elenca mai una chiave.

!!! warning
    Redis non dà avviso prima di maxmemory: al limite o evita le chiavi in silenzio o rifiuta ogni scrittura, a seconda di maxmemory-policy. Senza maxmemory cresce finché il kernel non lo uccide. DumbMonit avvisa al di sopra del 90% di maxmemory.

## Credenziali

| Credenziale | Campi |
|---|---|
| Utente ACL (consigliato) | L'utente creato sopra e la sua password, inviati come `AUTH user password`. |
| Solo password | Il valore `requirepass`, inviato come `AUTH password`. |
| Nessuna autenticazione | Per un server senza password. |

Indirizzo: un nome host o IP (`cache.lan`), `host:port`, o una stringa di connessione
(`redis://cache.lan:6380`; qualsiasi cosa dopo la porta viene ignorata, e le
credenziali scritte in essa non vengono mai utilizzate). Opzioni: porta (6379), TLS (spento),
verifica del certificato, e timeout per l'intero controllo (10 s). Con TLS attivo, il
certificato viene controllato rispetto alle autorità pubbliche incorporate in DumbMonit;
per un'autorità privata, abilita Accetta un certificato non verificabile.

Una password rifiutata (`WRONGPASS`, `NOAUTH`, `NOPERM`) viene mostrata come un errore
di credenziale e nessuno viene avvisato dell'errore stesso, ma DumbMonit allora non
raccoglie niente: la regola Device irraggiungibile continua a salire, come per qualsiasi
monitor che non può leggere, finché le credenziali non funzionano di nuovo. Redis
riavviato senza il suo utente ACL (utenti definiti con `ACL SETUSER` ma mai salvati)
causa esattamente questo.

In una distribuzione Redis Sentinel o Cluster, aggiungi ogni server che vuoi osservare:
ognuno segnala la sua memoria, client e persistenza, e un primary segnala le sue repliche.
Sentinel stesso non è coperto.

# MongoDB

Il database di documenti stesso: il replica set (membri, il loro stato, un primary
mancante, quanto indietro è ogni secondary), le connessioni rispetto al limite,
la cache WiredTiger, le operazioni e le asserzioni.

DumbMonit accede con un account che detiene solo il ruolo incorporato `clusterMonitor`,
esegue `serverStatus` e `replSetGetStatus` quando il server appartiene a un
replica set. Quel ruolo legge lo stato del server e della replica, e nessun
documento di nessun database. MongoDB 3.6 e successivi sono supportati; DumbMonit
parla il protocollo wire stesso, senza un driver.

I fallimenti che vale la pena osservare: un replica set che ha perso il suo primary
non accetta scritture finché un'elezione non ha successo; un secondary che rimane
indietro è quello che verrà eletto il giorno in cui il primary muore, senza le
scritture più recenti; al limite di connessione i nuovi client vengono rifiutati;
con più del 20% della cache WiredTiger sporco, le query devono evict le pagine
stesse e tutto rallenta.

## Cosa monitora

Tutte le metriche hanno il prefisso `dumbmonit_mongodb_`. I totali sono contatori:
i grafici e le regole li trasformano in rate.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version`, `process`, `storage_engine` |
| `role_info` | valore 1 | `role`: `primary`, `secondary`, `arbiter`, `standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | current rispetto a current più available | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | operazioni da quando è avviato | `op`: `insert`, `query`, `update`, `delete`, `getmore`, `command` |
| `asserts_total` | asserzioni da quando è avviato; `type` è `regular` (errori interni del server), `warning`, `msg`, `user` (errori restituiti ai client), `tripwire` o `rollovers`. `regular` e `user` sono quelli che meritano un grafico | `type` |
| `queued_operations`, `active_clients` | operazioni in attesa di un lock, client che ne eseguono una | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | la cache WiredTiger e quanto è piena | |
| `wiredtiger_cache_dirty_bytes`, `wiredtiger_cache_dirty_percent` | pagine modificate non ancora scritte | |
| `wiredtiger_cache_read_bytes_total` | byte letti dal disco nella cache | |
| `memory_resident_bytes` | | |
| `network_bytes_in_total`, `network_bytes_out_total` | | |
| `cursors_open`, `cursors_timed_out_total`, `documents_deleted_total` | | |
| `replset_info` | valore 1, in un replica set | `set`, `members` |
| `replset_primary_present` | 1 quando un membro è primary, 0 quando nessuno lo è | |
| `replset_member_state` | il numero dello stato del membro | `member`, `state`: `primary`, `secondary`, `arbiter`, `recovering`, `down`… |
| `replset_member_health` | 1 quando il membro è raggiungibile, 0 quando non lo è | `member` |
| `replset_member_lag_seconds` | quanto indietro è un secondary rispetto al primary | `member` |

Le [regole incorporate](../alerting/rules.md#databases-message-broker-and-security-engine)
che si applicano: MongoDB replica set senza primary, MongoDB membro irraggiungibile,
MongoDB replication lag, MongoDB connessioni vicine al limite, MongoDB cache sotto pressione,
MongoDB errori interni, più Device irraggiungibile.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la pagina
non interroga mai MongoDB. Dice cosa c'è che non va in una frase (replica set,
connessioni, cache, errori interni), poi le connessioni, le operazioni al secondo,
la cache usata, la memoria residente, le operazioni in coda e l'uptime, poi lo stato
e il lag di ogni membro del replica set e le operazioni al secondo per tipo.

## Crea un account MongoDB con il ruolo clusterMonitor

1. In mongosh, acceduto con un utente autorizzato a creare utenti, crea un account con solo il ruolo clusterMonitor incorporato. Legge lo stato del server e della replica, e nessun documento di nessun database.

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. DumbMonit accede con SCRAM-SHA-256, il predefinito per gli account creati su MongoDB 4.0 e successivi. Se hai creato l'account in un database diverso da admin, imposta il database di autenticazione qui sotto.

3. In DumbMonit, inserisci l'indirizzo di un mongod, ad esempio "db1.lan" o "db1.lan:27018". In un replica set, aggiungi ogni membro: ognuno segnala le proprie connessioni e cache, e uno qualsiasi segnala lo stato e il lag di tutti i membri. Se il server richiede TLS, abilita TLS qui sotto.

4. DumbMonit esegue solo serverStatus e replSetGetStatus. Non elenca mai una collection e non legge mai un documento.

!!! warning
    Un indirizzo mongodb+srv:// non viene risolto: inserisci l'host di ogni membro. MongoDB più vecchio di 3.6 e i router mongos non sono supportati.

## Credenziali

| Credenziale | Campi |
|---|---|
| Nome utente / password | L'account `clusterMonitor` creato sopra, acceduto con SCRAM-SHA-256. |
| Nessuna autenticazione | Per un server avviato senza controllo d'accesso. |

Un account che ha solo credenziali SCRAM-SHA-1 (creato prima di MongoDB 4.0 e
mai aggiornato) viene rifiutato: aggiorna la sua password una volta con
`db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})`.

Indirizzo: un nome host o IP (`db1.lan`), `host:port`, o una stringa di connessione
`mongodb://` con un singolo host. Opzioni: porta (27017), database di autenticazione
(`admin`), TLS (spento), verifica del certificato e timeout per l'intero controllo (10 s).

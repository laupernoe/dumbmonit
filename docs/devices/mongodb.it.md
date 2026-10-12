# MongoDB

Il database documentale stesso: il replica set (membri, il loro stato, un
primario mancante, quanto è indietro ogni secondario), le connessioni
rispetto al limite, la cache WiredTiger, operazioni e assert.

DumbMonit accede con un account che possiede solo il ruolo integrato
`clusterMonitor`, esegue `serverStatus`, e `replSetGetStatus` quando il
server appartiene a un replica set. Quel ruolo legge lo stato del server e
della replicazione, e nessun documento di alcun database. MongoDB 3.6 e
successivi sono supportati; DumbMonit parla direttamente il protocollo di
rete, senza un driver.

I guasti da tenere d'occhio: un replica set che ha perso il suo primario non
accetta alcuna scrittura finché un'elezione non riesce; un secondario che
rimane indietro è quello che verrà eletto il giorno in cui il primario
muore, senza le scritture più recenti; al limite delle connessioni i nuovi
client vengono rifiutati; con più del 20% della cache WiredTiger sporca, le
query devono evacuare pagine da sole e tutto si rallenta.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_mongodb_`. I totali sono
contatori: grafici e regole li trasformano in tassi.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version`, `process`, `storage_engine` |
| `role_info` | valore 1 | `role`: `primary`, `secondary`, `arbiter`, `standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | attuali rispetto ad attuali più disponibili | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | operazioni dall'avvio | `op`: `insert`, `query`, `update`, `delete`, `getmore`, `command` |
| `asserts_total` | assert dall'avvio; `regular` sono errori interni del server, `user` errori restituiti ai client | `type` |
| `queued_operations`, `active_clients` | operazioni in attesa di un lock, client che ne eseguono una | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | la cache WiredTiger e quanto è piena | |
| `wiredtiger_cache_dirty_bytes`, `wiredtiger_cache_dirty_percent` | pagine modificate non ancora scritte | |
| `wiredtiger_cache_read_bytes_total` | byte letti dal disco nella cache | |
| `memory_resident_bytes` | | |
| `network_bytes_in_total`, `network_bytes_out_total` | | |
| `cursors_open`, `cursors_timed_out_total`, `documents_deleted_total` | | |
| `replset_info` | valore 1, in un replica set | `set`, `members` |
| `replset_primary_present` | 1 quando un membro è primario, 0 quando nessuno lo è | |
| `replset_member_state` | il numero di stato del membro | `member`, `state`: `primary`, `secondary`, `arbiter`, `recovering`, `down`… |
| `replset_member_health` | 1 quando il membro è raggiungibile, 0 quando non lo è | `member` |
| `replset_member_lag_seconds` | quanto un secondario è indietro rispetto al primario | `member` |

Le [regole integrate](../alerting/rules.md#databases-message-broker-and-security-engine)
che si applicano: MongoDB replica set without primary, MongoDB member
unreachable, MongoDB replication lag, MongoDB connections near the limit,
MongoDB cache under pressure, MongoDB internal errors, più Device
unreachable.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai MongoDB. Indica cosa non va in una frase (replica
set, connessioni, cache, errori interni), poi le connessioni, le operazioni
al secondo, la cache usata, la memoria residente, le operazioni in coda e
l'uptime, poi lo stato e il ritardo di ogni membro del replica set e le
operazioni al secondo per tipo.

## Creare un account MongoDB con il ruolo clusterMonitor

1. In mongosh, autenticato con un utente autorizzato a creare utenti, crea un account con solo il ruolo integrato clusterMonitor. Legge lo stato del server e della replicazione, e nessun documento di alcun database.

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. DumbMonit accede con SCRAM-SHA-256, il valore predefinito per gli account creati su MongoDB 4.0 e successivi. Se hai creato l'account in un database diverso da admin, imposta Authentication database più sotto.

3. In DumbMonit, inserisci l'indirizzo di un mongod, per esempio "db1.lan" oppure "db1.lan:27018". In un replica set, aggiungi ogni membro: ciascuno riporta le proprie connessioni e cache, e qualsiasi di essi riporta lo stato e il ritardo di tutti i membri. Se il server richiede TLS, spunta TLS più sotto.

4. DumbMonit esegue solo serverStatus e replSetGetStatus. Non elenca mai una collection e non legge mai un documento.

!!! warning
    Un indirizzo mongodb+srv:// non viene risolto: inserisci invece l'host di ogni membro. MongoDB più vecchio di 3.6 e i router mongos non sono supportati.

## Credenziali

| Credenziale | Campi |
|---|---|
| Username / password | L'account `clusterMonitor` creato sopra, autenticato con SCRAM-SHA-256. |
| No authentication | Per un server avviato senza controllo degli accessi. |

Un account che ha solo credenziali SCRAM-SHA-1 (creato prima di MongoDB 4.0
e mai aggiornato) viene rifiutato: aggiorna la sua password una volta con
`db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})`.

Indirizzo: un nome host o IP (`db1.lan`), `host:porta`, oppure una stringa di
connessione `mongodb://` con un singolo host. Opzioni: porta (27017),
database di autenticazione (`admin`), TLS (disattivato), verifica del
certificato, e timeout per l'intero controllo (10 s).

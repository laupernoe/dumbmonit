# MongoDB

Die Dokumentendatenbank selbst: das Replica Set (Mitglieder, ihr
Zustand, ein fehlender Primary, wie weit jeder Secondary zurückliegt),
Verbindungen gegen das Limit, der WiredTiger-Cache, Operationen und
Assertions.

DumbMonit loggt sich mit einem Konto ein, das nur die eingebaute Rolle
`clusterMonitor` hält, führt `serverStatus` aus, und `replSetGetStatus`,
wenn der Server zu einem Replica Set gehört. Diese Rolle liest den
Zustand des Servers und der Replikation, und kein Dokument irgendeiner
Datenbank. MongoDB 3.6 und später werden abgedeckt; DumbMonit spricht
das Wire-Protokoll selbst, ohne Treiber.

Die beobachtenswerten Fehler: Ein Replica Set, das seinen Primary
verloren hat, nimmt keinen Schreibvorgang an, bis eine Wahl gelingt; ein
zurückfallender Secondary ist derjenige, der an dem Tag gewählt wird, an
dem der Primary stirbt, ohne die neuesten Schreibvorgänge; am
Verbindungslimit werden neue Clients abgelehnt; mit mehr als 20 % dirty
im WiredTiger-Cache müssen Abfragen Seiten selbst verdrängen, und alles
wird langsam.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_mongodb_`. Totals sind Zähler:
Diagramme und Regeln verwandeln sie in Raten.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version`, `process`, `storage_engine` |
| `role_info` | Wert 1 | `role`: `primary`, `secondary`, `arbiter`, `standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | current gegenüber current plus available | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | Operationen seit Start | `op`: `insert`, `query`, `update`, `delete`, `getmore`, `command` |
| `asserts_total` | Assertions seit Start; `regular` sind interne Serverfehler, `user` Fehler, die an Clients zurückgegeben werden | `type` |
| `queued_operations`, `active_clients` | auf eine Sperre wartende Operationen, Clients, die eine ausführen | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | der WiredTiger-Cache und wie voll er ist | |
| `wiredtiger_cache_dirty_bytes`, `wiredtiger_cache_dirty_percent` | geänderte, noch nicht geschriebene Seiten | |
| `wiredtiger_cache_read_bytes_total` | von der Platte in den Cache gelesene Bytes | |
| `memory_resident_bytes` | | |
| `network_bytes_in_total`, `network_bytes_out_total` | | |
| `cursors_open`, `cursors_timed_out_total`, `documents_deleted_total` | | |
| `replset_info` | Wert 1, in einem Replica Set | `set`, `members` |
| `replset_primary_present` | 1, wenn ein Mitglied Primary ist, 0, wenn keines es ist | |
| `replset_member_state` | die Zustandsnummer des Mitglieds | `member`, `state`: `primary`, `secondary`, `arbiter`, `recovering`, `down`… |
| `replset_member_health` | 1, wenn das Mitglied erreichbar ist, 0, wenn nicht | `member` |
| `replset_member_lag_seconds` | wie weit ein Secondary hinter dem Primary liegt | `member` |

Die geltenden [eingebauten Regeln](../alerting/rules.md#databases-message-broker-and-security-engine):
MongoDB replica set without primary, MongoDB member unreachable, MongoDB
replication lag, MongoDB connections near the limit, MongoDB cache
under pressure, MongoDB internal errors, plus Device unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt MongoDB nie ab. Es sagt in einem Satz, was nicht
stimmt (Replica Set, Verbindungen, Cache, interne Fehler), dann die
Verbindungen, Operationen pro Sekunde, genutzten Cache, residenten
Speicher, wartende Operationen und Uptime, dann den Zustand und die
Verzögerung jedes Replica-Set-Mitglieds und die Operationen pro Sekunde
nach Art.

## Ein MongoDB-Konto mit der Rolle clusterMonitor erstellen

1. Erstellen Sie in mongosh, eingeloggt mit einem Benutzer, der
   Benutzer anlegen darf, ein Konto mit nur der eingebauten Rolle
   clusterMonitor. Sie liest den Server- und Replikationsstatus, und
   kein Dokument irgendeiner Datenbank.

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. DumbMonit loggt sich mit SCRAM-SHA-256 ein, dem Standard für auf
   MongoDB 4.0 und später erstellte Konten. Haben Sie das Konto in
   einer anderen Datenbank als admin erstellt, setzen Sie unten
   Authentication database.

3. Geben Sie in DumbMonit die Adresse eines mongod ein, zum Beispiel
   "db1.lan" oder "db1.lan:27018". In einem Replica Set fügen Sie jedes
   Mitglied hinzu: Jedes meldet seine eigenen Verbindungen und Cache,
   und jedes von ihnen meldet den Zustand und die Verzögerung aller
   Mitglieder. Verlangt der Server TLS, kreuzen Sie unten TLS an.

4. DumbMonit führt nur serverStatus und replSetGetStatus aus. Es listet
   nie eine Collection und liest nie ein Dokument.

!!! warning
    Eine mongodb+srv://-Adresse wird nicht aufgelöst: Geben Sie
    stattdessen den Host jedes Mitglieds ein. MongoDB älter als 3.6 und
    mongos-Router werden nicht unterstützt.

## Credentials

| Credential | Felder |
|---|---|
| Benutzername / Passwort | Das oben erstellte `clusterMonitor`-Konto, eingeloggt mit SCRAM-SHA-256. |
| Keine Authentifizierung | Für einen ohne Zugriffskontrolle gestarteten Server. |

Ein Konto, das nur SCRAM-SHA-1-Zugangsdaten hat (vor MongoDB 4.0
erstellt und nie aktualisiert), wird abgelehnt: Aktualisieren Sie sein
Passwort einmal mit
`db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})`.

Adresse: ein Hostname oder eine IP (`db1.lan`), `host:port`, oder ein
`mongodb://`-Connection-String mit einem einzelnen Host. Optionen: Port
(27017), Authentifizierungsdatenbank (`admin`), TLS (aus),
Zertifikatsprüfung, und Timeout für die gesamte Prüfung (10 s).

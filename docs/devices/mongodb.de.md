# MongoDB {#mongodb}

Die Dokumentendatenbank selbst: das Replica Set (Mitglieder, ihr Zustand, ein fehlender Primary,
wie weit jeder Secondary zurückliegt), Verbindungen gegenüber dem Limit, der WiredTiger-Cache,
Operationen und Assertions.

DumbMonit meldet sich mit einem Konto an, das nur die eingebaute Rolle `clusterMonitor` besitzt,
führt `serverStatus` aus und `replSetGetStatus`, wenn der Server zu einem Replica Set gehört.
Diese Rolle liest den Zustand des Servers und der Replikation, aber kein Dokument irgendeiner
Datenbank. MongoDB 3.6 und neuer werden abgedeckt; DumbMonit spricht das Wire-Protokoll selbst,
ohne Treiber.

Die Fehler, auf die es ankommt: Ein Replica Set, das seinen Primary verloren hat, nimmt keine
Schreibzugriffe an, bis eine Wahl gelingt; ein Secondary, der zurückfällt, ist der, der an dem
Tag gewählt wird, an dem der Primary stirbt, ohne die neuesten Schreibvorgänge; am
Verbindungslimit werden neue Clients abgewiesen; sind mehr als 20 % des WiredTiger-Caches
„dirty“, müssen Abfragen selbst Seiten verdrängen, und alles wird langsamer.

## Was überwacht wird {#what-it-watches}

Alle Metriken haben das Präfix `dumbmonit_mongodb_`. Summen sind Zähler: Diagramme und Regeln
wandeln sie in Raten um.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version`, `process`, `storage_engine` |
| `role_info` | Wert 1 | `role`: `primary`, `secondary`, `arbiter`, `standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | aktuell im Verhältnis zu aktuell plus verfügbar | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | Operationen seit dem Start | `op`: `insert`, `query`, `update`, `delete`, `getmore`, `command` |
| `asserts_total` | Assertions seit dem Start; `type` ist `regular` (interne Serverfehler), `warning`, `msg`, `user` (an Clients zurückgegebene Fehler), `tripwire` oder `rollovers`. `regular` und `user` verdienen ein Diagramm | `type` |
| `queued_operations`, `active_clients` | auf eine Sperre wartende Operationen, Clients, die eine ausführen | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | der WiredTiger-Cache und wie voll er ist | |
| `wiredtiger_cache_dirty_bytes`, `wiredtiger_cache_dirty_percent` | geänderte, noch nicht geschriebene Seiten | |
| `wiredtiger_cache_read_bytes_total` | von der Festplatte in den Cache gelesene Bytes | |
| `memory_resident_bytes` | | |
| `network_bytes_in_total`, `network_bytes_out_total` | | |
| `cursors_open`, `cursors_timed_out_total`, `documents_deleted_total` | | |
| `replset_info` | Wert 1, in einem Replica Set | `set`, `members` |
| `replset_primary_present` | 1, wenn ein Mitglied Primary ist, 0, wenn keines | |
| `replset_member_state` | die Zustandsnummer des Mitglieds | `member`, `state`: `primary`, `secondary`, `arbiter`, `recovering`, `down`… |
| `replset_member_health` | 1, wenn das Mitglied erreichbar ist, 0, wenn nicht | `member` |
| `replset_member_lag_seconds` | wie weit ein Secondary hinter dem Primary liegt | `member` |

Die zutreffenden [integrierten Regeln](../alerting/rules.md#databases-message-broker-and-security-engine):
MongoDB-Replica-Set ohne Primary, MongoDB-Mitglied nicht erreichbar, MongoDB-Replikationsverzug,
MongoDB-Verbindungen nahe am Limit, MongoDB-Cache unter Druck, MongoDB-interne Fehler, außerdem
Gerät nicht erreichbar.

## Die Geräteseite {#the-device-page}

Das Panel über den Diagrammen liest, was die Prüfung gespeichert hat; das Öffnen der Seite
fragt MongoDB nie ab. Es nennt in einem Satz, was nicht stimmt (Replica Set, Verbindungen,
Cache, interne Fehler), dann die Verbindungen, Operationen pro Sekunde, genutzten Cache,
resident genutzten Speicher, wartende Operationen und die Laufzeit, dann Zustand und Verzug
jedes Replica-Set-Mitglieds und die Operationen pro Sekunde nach Typ.

## Ein MongoDB-Konto mit der Rolle clusterMonitor erstellen {#create-a-mongodb-account-with-the-clustermonitor-role}

1. Erstellen Sie in mongosh, angemeldet mit einem Benutzer, der Benutzer anlegen darf, ein Konto nur mit der eingebauten Rolle clusterMonitor. Sie liest den Server- und Replikationsstatus, aber kein Dokument irgendeiner Datenbank.

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. DumbMonit meldet sich mit SCRAM-SHA-256 an, dem Standard für auf MongoDB 4.0 und neuer erstellte Konten. Haben Sie das Konto in einer anderen Datenbank als admin angelegt, setzen Sie unten Authentifizierungsdatenbank.

3. Geben Sie in DumbMonit die Adresse eines mongod ein, zum Beispiel „db1.lan“ oder „db1.lan:27018“. Fügen Sie in einem Replica Set jedes Mitglied hinzu: Jedes meldet seine eigenen Verbindungen und seinen Cache, und jedes meldet Zustand und Verzug aller Mitglieder. Verlangt der Server TLS, haken Sie unten TLS an.

4. DumbMonit führt nur serverStatus und replSetGetStatus aus. Es listet nie eine Collection auf und liest nie ein Dokument.

!!! warning
    Eine Adresse mongodb+srv:// wird nicht aufgelöst: Geben Sie stattdessen den Host jedes Mitglieds ein. MongoDB älter als 3.6 und mongos-Router werden nicht unterstützt.

## Zugangsdaten {#credentials}

| Zugangsdaten | Felder |
|---|---|
| Benutzername / Passwort | Das oben erstellte `clusterMonitor`-Konto, angemeldet mit SCRAM-SHA-256. |
| Keine Authentifizierung | Für einen ohne Zugriffskontrolle gestarteten Server. |

Ein Konto, das nur SCRAM-SHA-1-Zugangsdaten hat (vor MongoDB 4.0 erstellt und nie aktualisiert),
wird abgewiesen: Aktualisieren Sie sein Passwort einmalig mit
`db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})`.

Adresse: ein Hostname oder eine IP (`db1.lan`), `host:port` oder eine `mongodb://`-Verbindungs-
zeichenfolge mit einem einzigen Host. Optionen: Port (27017), Authentifizierungsdatenbank
(`admin`), TLS (aus), Zertifikatsprüfung und Timeout für die gesamte Prüfung (10 s).

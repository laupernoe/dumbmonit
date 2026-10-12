# Redis und Valkey

Der In-Memory-Store selbst: Speicher gegen `maxmemory`, abgelehnte
Verbindungen, der Replikationslink und die Verzögerung jeder Replica,
fehlgeschlagene Snapshots und Append-Only-File-Schreibvorgänge, und die
Größe des Keyspace.

DumbMonit öffnet eine Verbindung, sendet `AUTH`, dann `INFO`, und
schließt sie. Das genutzte Konto kann nur `INFO` und `PING` ausführen
und nichts sonst: Es kann keinen Key lesen, schreiben oder auch nur
auflisten. Redis 6 und später, Valkey und die Forks, die `INFO`
genauso beantworten (KeyDB, Dragonfly), werden abgedeckt.

Die beobachtenswerten Fehler sind die stillen. Bei `maxmemory` entfernt
Redis entweder Keys, ohne jemandem Bescheid zu sagen, oder lehnt jeden
Schreibvorgang mit `OOM command not allowed` ab, je nach
`maxmemory-policy`; ohne `maxmemory` wächst es, bis der Kernel es
tötet. Ein fehlgeschlagener Snapshot lässt Redis Schreibvorgänge
ablehnen, solange `stop-writes-on-bgsave-error` an ist, was es
standardmäßig ist. Eine Replica, deren Link zu ihrem Primary down ist,
beantwortet weiter Lesevorgänge, mit veralteten Daten.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_redis_`. Totals sind Zähler:
Diagramme und Regeln verwandeln sie in Raten.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `product` (`redis`, `valkey`), `version`, `mode` |
| `role_info` | Wert 1 | `role`: `master` oder `replica` |
| `memory_used_bytes`, `memory_rss_bytes`, `memory_peak_bytes`, `memory_fragmentation_ratio` | Speicher, wie Redis ihn zählt, und wie das System | |
| `memory_max_bytes`, `memory_used_percent` | `maxmemory` und der genutzte Anteil; fehlt, wenn `maxmemory` 0 ist (kein Limit) | |
| `maxmemory_policy_info` | Wert 1 | `policy` |
| `connected_clients`, `blocked_clients`, `max_clients` | | |
| `rejected_connections_total` | abgelehnte Verbindungen, weil `maxclients` erreicht wurde | |
| `evicted_keys_total`, `expired_keys_total` | bei `maxmemory` entfernte Keys, abgelaufene Keys | |
| `keyspace_hits_total`, `keyspace_misses_total` | Lookups, die ihren Key fanden, und die, die nicht | |
| `commands_processed_total`, `connections_received_total`, `ops_per_second` | | |
| `error_replies_total`, `acl_denied_auth_total` | gesendete Fehlerantworten, und abgelehnte Logins (ein erratenes Passwort zeigt sich hier) | |
| `keys`, `keys_expiring` | Keys in allen Datenbanken, und die mit einem Ablauf; immer vorhanden, auch bei null | |
| `db_keys` | Keys pro Datenbank | `db` |
| `rdb_last_save_ok` | 0, wenn der letzte Snapshot (`BGSAVE`) fehlgeschlagen ist | |
| `rdb_changes_since_last_save`, `rdb_last_save_timestamp_seconds` | | |
| `aof_enabled`, `aof_last_write_ok`, `aof_last_rewrite_ok` | Append-Only-File; die beiden Status nur, wenn es an ist | |
| `loading` | 1, während Redis beim Start seinen Datensatz lädt | |
| `connected_replicas` | an diesen Primary angehängte Replicas | |
| `replica_online`, `replica_lag_seconds`, `replica_lag_bytes` | vom Primary aus gesehen, pro Replica: online oder nicht, Sekunden seit der letzten Bestätigung, zurückliegende Bytes | `replica` (`ip:port`) |
| `master_link_up`, `master_last_io_seconds`, `master_link_down_seconds`, `master_sync_in_progress` | auf einer Replica: ihr Link zum Primary | |
| `uptime_seconds`, `cluster_enabled` | | |

Die geltenden [eingebauten Regeln](../alerting/rules.md#databases-message-broker-and-security-engine):
Redis memory near maxmemory, Redis refusing connections, Redis
replication link down, Redis replica lagging, Redis snapshot failed,
Redis append-only file write failed, plus Device unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt Redis nie ab. Es sagt in einem Satz, was nicht
stimmt (Replikation, Speicher gegen `maxmemory` und die am Limit
geltende Richtlinie, Persistenz, abgelehnte Verbindungen), dann die
Keys, Speicher, Clients, Befehle pro Sekunde, Trefferquote, in den
letzten 24 Stunden entfernte Keys und Uptime, dann die Keys pro
Datenbank und die Verzögerung jeder Replica.

## Einen Redis-Benutzer erstellen, der nur INFO ausführen darf

1. Erstellen Sie auf Redis 6 oder später, oder Valkey, einen Benutzer,
   der nur INFO und PING ausführen darf und nichts sonst, ohne Zugriff
   auf irgendeinen Key. In redis-cli, eingeloggt mit einem zur
   ACL-Verwaltung berechtigten Benutzer:

    ```
    ACL SETUSER dumbmonit on >a-long-password -@all +info +ping
    ```

2. Machen Sie es dauerhaft: ACL SAVE, wenn der Server seine Benutzer
   aus einer aclfile lädt, sonst CONFIG REWRITE.

    ```
    ACL SAVE
    ```

3. Auf Redis 5 oder älter, oder nur mit gesetztem requirepass, wählen
   Sie unten Nur Passwort. Dieses Passwort trägt alle Rechte: Bevorzugen
   Sie einen ACL-Benutzer, wann immer der Server einen unterstützt.

4. Geben Sie in DumbMonit die Serveradresse ein, zum Beispiel
   "cache.lan" oder "cache.lan:6380". Akzeptiert der Server auf diesem
   Port nur TLS (tls-port), kreuzen Sie unten TLS an.

5. DumbMonit sendet AUTH, dann INFO. Es liest, schreibt oder listet nie
   einen Key.

!!! warning
    Redis gibt vor maxmemory keine Warnung: am Limit entfernt es
    entweder stillschweigend Keys oder lehnt jeden Schreibvorgang ab,
    je nach maxmemory-policy. Ohne maxmemory wächst es, bis der Kernel
    es tötet. DumbMonit warnt über 90 % von maxmemory.

## Credentials

| Credential | Felder |
|---|---|
| ACL-Benutzer (empfohlen) | Der oben erstellte Benutzer und sein Passwort, als `AUTH user password` gesendet. |
| Nur Passwort | Der Wert von `requirepass`, als `AUTH password` gesendet. |
| Keine Authentifizierung | Für einen Server ganz ohne Passwort. |

Adresse: ein Hostname oder eine IP (`cache.lan`), `host:port`, oder ein
Connection-String (`redis://cache.lan:6380`; alles nach dem Port wird
ignoriert, und darin geschriebene Zugangsdaten werden nie genutzt).
Optionen: Port (6379), TLS (aus), Zertifikatsprüfung, und Timeout für
die gesamte Prüfung (10 s). Mit aktiviertem TLS wird das Zertifikat
gegen die in DumbMonit eingebauten öffentlichen Autoritäten geprüft;
für eine private Autorität kreuzen Sie "Accept an unverifiable
certificate" an.

Fügen Sie in einem Redis-Sentinel- oder Cluster-Deployment jeden zu
beobachtenden Server hinzu: Jeder meldet seinen eigenen Speicher,
Clients und Persistenz, und ein Primary meldet seine Replicas. Sentinel
selbst wird nicht abgedeckt.

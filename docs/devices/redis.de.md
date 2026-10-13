# Redis und Valkey {#redis-and-valkey}

Der In-Memory-Speicher selbst: Speicher gegenüber `maxmemory`, abgewiesene Verbindungen, die
Replikationsverbindung und der Verzug jedes Replikats, fehlgeschlagene Snapshots und
Append-only-Datei-Schreibvorgänge sowie die Größe des Keyspace.

DumbMonit öffnet eine Verbindung, sendet `AUTH`, dann `INFO`, und schließt sie. Das verwendete
Konto kann `INFO` und `PING` ausführen und sonst nichts: Es kann keinen Schlüssel lesen,
schreiben oder auch nur auflisten. Redis 6 und neuer, Valkey und die Forks, die `INFO` genauso
beantworten (KeyDB, Dragonfly), werden abgedeckt.

Die Fehler, auf die es ankommt, sind die stillen. Bei `maxmemory` verdrängt Redis entweder
Schlüssel, ohne es jemandem zu sagen, oder weist je nach `maxmemory-policy` jeden Schreibzugriff
mit `OOM command not allowed` ab; ohne `maxmemory` wächst es, bis der Kernel es beendet. Ein
fehlgeschlagener Snapshot lässt Redis Schreibzugriffe abweisen, solange
`stop-writes-on-bgsave-error` aktiv ist, was standardmäßig der Fall ist. Ein Replikat, dessen
Verbindung zum Primary unterbrochen ist, beantwortet weiter Lesezugriffe, mit veralteten Daten.

## Was überwacht wird {#what-it-watches}

Alle Metriken haben das Präfix `dumbmonit_redis_`. Summen sind Zähler: Diagramme und Regeln
wandeln sie in Raten um.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `product` (`redis`, `valkey`), `version`, `mode` |
| `role_info` | Wert 1 | `role`: `master` oder `replica` |
| `memory_used_bytes`, `memory_rss_bytes`, `memory_peak_bytes`, `memory_fragmentation_ratio` | Speicher, wie Redis ihn zählt und wie das System ihn zählt | |
| `memory_max_bytes`, `memory_used_percent` | `maxmemory` und der genutzte Anteil; fehlt, wenn `maxmemory` 0 ist (kein Limit) | |
| `maxmemory_policy_info` | Wert 1 | `policy` |
| `connected_clients`, `blocked_clients`, `max_clients` | | |
| `rejected_connections_total` | abgewiesene Verbindungen, weil `maxclients` erreicht war | |
| `evicted_keys_total`, `expired_keys_total` | bei `maxmemory` verdrängte Schlüssel, abgelaufene Schlüssel | |
| `keyspace_hits_total`, `keyspace_misses_total` | Lookups, die ihren Schlüssel fanden, und solche, die ihn nicht fanden | |
| `commands_processed_total`, `connections_received_total`, `ops_per_second` | | |
| `error_replies_total`, `acl_denied_auth_total` | gesendete Fehlerantworten und abgewiesene Anmeldungen (ein Passwort, das erraten wird, zeigt sich hier) | |
| `keys`, `keys_expiring` | Schlüssel in allen Datenbanken und solche mit Ablaufzeit; immer vorhanden, auch bei null | |
| `db_keys` | Schlüssel pro Datenbank | `db` |
| `rdb_last_save_ok` | 0, wenn der letzte Snapshot (`BGSAVE`) fehlgeschlagen ist | |
| `rdb_changes_since_last_save`, `rdb_last_save_timestamp_seconds` | | |
| `aof_enabled`, `aof_last_write_ok`, `aof_last_rewrite_ok` | Append-only-Datei; die beiden Status nur, wenn sie aktiv ist | |
| `loading` | 1, während Redis beim Start seinen Datensatz lädt | |
| `connected_replicas` | an diesen Primary angeschlossene Replikate | |
| `replica_online`, `replica_lag_seconds`, `replica_lag_bytes` | vom Primary aus gesehen, pro Replikat: online oder nicht, Sekunden seit der letzten Bestätigung, Rückstand in Bytes | `replica` (`ip:port`) |
| `master_link_up`, `master_last_io_seconds`, `master_link_down_seconds`, `master_sync_in_progress` | auf einem Replikat: seine Verbindung zum Primary | |
| `uptime_seconds`, `cluster_enabled` | | |

Die zutreffenden [integrierten Regeln](../alerting/rules.md#databases-message-broker-and-security-engine):
Redis-Speicher nahe maxmemory, Redis weist Verbindungen ab, Redis-Replikationsverbindung
unterbrochen, Redis-Replikat im Rückstand, Redis-Snapshot fehlgeschlagen, Redis-Append-only-
Datei-Schreibvorgang fehlgeschlagen, außerdem Gerät nicht erreichbar.

## Die Geräteseite {#the-device-page}

Das Panel über den Diagrammen liest, was die Prüfung gespeichert hat; das Öffnen der Seite
fragt Redis nie ab. Es nennt in einem Satz, was nicht stimmt (Replikation, Speicher gegenüber
`maxmemory` und die am Limit geltende Richtlinie, Persistenz, abgewiesene Verbindungen), dann
Schlüssel, Speicher, Clients, Befehle pro Sekunde, Trefferquote, in den letzten 24 Stunden
verdrängte Schlüssel und Laufzeit, dann die Schlüssel pro Datenbank und den Verzug jedes
Replikats.

## Einen Redis-Benutzer erstellen, der nur INFO ausführen darf {#create-a-redis-user-that-may-only-run-info}

1. Erstellen Sie unter Redis 6 oder neuer oder Valkey einen Benutzer, der INFO und PING ausführen darf und sonst nichts, ohne Zugriff auf irgendeinen Schlüssel. In redis-cli, angemeldet mit einem Benutzer, der ACLs verwalten darf:

    ```
    ACL SETUSER dumbmonit on >a-long-password -@all +info +ping
    ```

2. Machen Sie es dauerhaft: ACL SAVE, wenn der Server seine Benutzer aus einer aclfile lädt, sonst CONFIG REWRITE.

    ```
    ACL SAVE
    ```

3. Wählen Sie unter Redis 5 oder älter oder wenn nur requirepass gesetzt ist unten Nur Passwort. Dieses Passwort trägt alle Rechte: Bevorzugen Sie einen ACL-Benutzer, wann immer der Server einen unterstützt.

4. Geben Sie in DumbMonit die Serveradresse ein, zum Beispiel „cache.lan“ oder „cache.lan:6380“. Akzeptiert der Server auf diesem Port nur TLS (tls-port), haken Sie unten TLS an.

5. DumbMonit sendet AUTH, dann INFO. Es liest, schreibt oder listet nie einen Schlüssel.

!!! warning
    Redis warnt nicht vor maxmemory: Am Limit verdrängt es entweder still Schlüssel oder weist je nach maxmemory-policy jeden Schreibzugriff ab. Ohne maxmemory wächst es, bis der Kernel es beendet. DumbMonit warnt ab 90 % von maxmemory.

## Zugangsdaten {#credentials}

| Zugangsdaten | Felder |
|---|---|
| ACL-Benutzer (empfohlen) | Der oben erstellte Benutzer und sein Passwort, gesendet als `AUTH user password`. |
| Nur Passwort | Der Wert von `requirepass`, gesendet als `AUTH password`. |
| Keine Authentifizierung | Für einen Server ganz ohne Passwort. |

Adresse: ein Hostname oder eine IP (`cache.lan`), `host:port` oder eine Verbindungszeichenfolge
(`redis://cache.lan:6380`; alles nach dem Port wird ignoriert, und darin geschriebene
Zugangsdaten werden nie verwendet). Optionen: Port (6379), TLS (aus), Zertifikatsprüfung und
Timeout für die gesamte Prüfung (10 s). Bei aktivem TLS wird das Zertifikat gegen die in
DumbMonit eingebauten öffentlichen Zertifizierungsstellen geprüft; für eine private Stelle haken
Sie Nicht prüfbares Zertifikat akzeptieren an.

Ein abgelehntes Passwort (`WRONGPASS`, `NOAUTH`, `NOPERM`) wird als Zugangsdatenfehler angezeigt,
und über den Fehler selbst wird niemand benachrichtigt, aber DumbMonit sammelt dann nichts mehr:
Die Regel Gerät nicht erreichbar löst weiter aus, wie bei jedem Gerät, das es nicht lesen kann,
bis die Zugangsdaten wieder funktionieren. Genau das verursacht ein Redis-Neustart ohne seinen
ACL-Benutzer (mit `ACL SETUSER` definierte, aber nie gespeicherte Benutzer).

Fügen Sie bei einem Redis-Sentinel- oder Cluster-Einsatz jeden Server hinzu, den Sie überwachen
möchten: Jeder meldet seinen eigenen Speicher, seine Clients und Persistenz, und ein Primary
meldet seine Replikate. Sentinel selbst wird nicht abgedeckt.

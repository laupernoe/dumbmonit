-- Destinations vers lesquelles le serveur expédie ses propres journaux
-- (Graylog, VictoriaLogs, Loki, Elasticsearch, syslog). `settings` est un objet
-- JSON renvoyé tel quel par l'API ; `secrets` est chiffré (AES-256-GCM) et n'en
-- ressort jamais.
CREATE TABLE log_destinations (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL,
    kind       TEXT    NOT NULL,
    enabled    INTEGER NOT NULL DEFAULT 1,
    min_level  TEXT    NOT NULL DEFAULT 'info',
    settings   TEXT    NOT NULL DEFAULT '{}',
    secrets    BLOB,
    created_at TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

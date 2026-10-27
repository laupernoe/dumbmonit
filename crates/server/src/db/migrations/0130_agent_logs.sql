-- Collecte de journaux par l'agent : réglage par machine, piloté depuis
-- l'interface (le serveur le joint à l'accusé de réception des lots de mesures).
-- Désactivée par défaut ; les lignes reçues partent dans VictoriaLogs, rien
-- n'est gardé ici hormis de quoi dire « dernière réception ».
CREATE TABLE agent_log_settings (
    target_id        INTEGER PRIMARY KEY REFERENCES targets(id) ON DELETE CASCADE,
    enabled          INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    min_level        TEXT    NOT NULL DEFAULT 'warn'
                             CHECK (min_level IN ('error', 'warn', 'info', 'debug')),
    journald         INTEGER NOT NULL DEFAULT 1 CHECK (journald IN (0, 1)),
    files            TEXT    NOT NULL DEFAULT '[]',   -- tableau JSON de motifs
    eventlog         TEXT    NOT NULL DEFAULT '[]',   -- tableau JSON de canaux Windows
    last_received_at TEXT,                            -- UTC sans suffixe
    received_total   INTEGER NOT NULL DEFAULT 0,
    updated_at       TEXT    NOT NULL DEFAULT (datetime('now'))
);

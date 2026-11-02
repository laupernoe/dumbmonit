-- Mode sentinelle : un agent désigné surveille que le serveur répond et alerte
-- lui-même quand ce n'est plus le cas (voir docs/install/sentinel.md).
--
-- Le canal de notification (webhook HTTPS ou sujet ntfy) est un secret : il est
-- stocké chiffré par la clé d'instance (JSON de `SentinelChannel`) et ne
-- ressort jamais par l'API, seulement vers l'agent désigné, sur le canal chiffré.
CREATE TABLE agent_sentinel (
    target_id     INTEGER PRIMARY KEY REFERENCES targets(id) ON DELETE CASCADE,
    enabled       INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    urls          TEXT    NOT NULL DEFAULT '[]',   -- tableau JSON d'adresses HTTPS
    interval_secs INTEGER NOT NULL DEFAULT 60,
    threshold     INTEGER NOT NULL DEFAULT 3,
    channel_enc   BLOB,                            -- chiffré ; NULL : pas de canal
    updated_at    TEXT    NOT NULL DEFAULT (datetime('now'))
);

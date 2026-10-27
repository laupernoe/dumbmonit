-- Sauvegarde des configurations réseau (`crates/server/src/netconfig`).
--
-- `netconfig_state` : une ligne par équipement. `host_key` est l'empreinte
-- SHA-256 de la clé d'hôte SSH mémorisée au premier relevé (confiance au
-- premier usage) ; `host_key_seen` celle, différente, qu'un relevé a refusée
-- — la connexion est alors coupée avant l'authentification et l'alerte
-- « SSH host key changed » se lève jusqu'à ce qu'un administrateur accepte la
-- nouvelle clé. `running_since` : verrou d'un relevé en cours (tenu pour
-- périmé au bout d'un quart d'heure). `last_changed` : le dernier relevé
-- réussi a produit une nouvelle version (hors toute première version).
CREATE TABLE netconfig_state (
    target_id          INTEGER PRIMARY KEY REFERENCES targets(id) ON DELETE CASCADE,
    host_key           TEXT,
    host_key_pinned_at TEXT,
    host_key_seen      TEXT,
    host_key_seen_at   TEXT,
    running_since      TEXT,
    last_attempt_at    TEXT,
    last_success_at    TEXT,
    last_error         TEXT,
    last_error_kind    TEXT,
    last_changed       INTEGER NOT NULL DEFAULT 0
);

-- `netconfig_versions` : une version par configuration différente (après
-- retrait des lignes volatiles du profil). `content_enc` : le texte relevé,
-- compressé (gzip) puis chiffré avec la clé d'instance (AES-256-GCM).
-- `digest` : HMAC-SHA256 de la forme comparable, sous une sous-clé de
-- l'instance, jamais une empreinte nue d'un texte qui contient des secrets.
-- `last_seen_at` : dernier relevé identique. AUTOINCREMENT : un numéro de
-- version n'est jamais réutilisé.
CREATE TABLE netconfig_versions (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    target_id    INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    fetched_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    last_seen_at TEXT    NOT NULL DEFAULT (datetime('now')),
    profile      TEXT    NOT NULL,
    digest       TEXT    NOT NULL,
    size_bytes   INTEGER NOT NULL,
    lines        INTEGER NOT NULL,
    added        INTEGER NOT NULL DEFAULT 0,
    removed      INTEGER NOT NULL DEFAULT 0,
    content_enc  BLOB    NOT NULL
);

CREATE INDEX netconfig_versions_target ON netconfig_versions (target_id, id);

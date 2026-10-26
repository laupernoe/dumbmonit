-- Clés d'accès (passkeys, WebAuthn) : une ligne par authentificateur enregistré.
--
-- Seule la partie publique est stockée : la clé privée ne quitte jamais
-- l'authentificateur, une copie de la base ne permet donc de se connecter
-- nulle part. `credential_id` identifie la clé auprès du navigateur, la clé
-- publique est un point P-256 non compressé (65 octets, algorithme ES256).
CREATE TABLE passkeys (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Nom choisi par l'utilisateur (« Téléphone », « YubiKey »).
    name            TEXT    NOT NULL,
    credential_id   BLOB    NOT NULL UNIQUE,
    public_key      BLOB    NOT NULL,
    -- Dernier compteur vu : un compteur qui recule trahit une clé clonée. Les
    -- clés synchronisées renvoient toujours 0, ce qui est accepté.
    sign_count      INTEGER NOT NULL DEFAULT 0,
    -- Drapeaux « backup eligible » / « backup state » : la clé est-elle copiée
    -- dans un trousseau synchronisé ?
    backup_eligible INTEGER NOT NULL DEFAULT 0,
    backup_state    INTEGER NOT NULL DEFAULT 0,
    transports      TEXT    NOT NULL DEFAULT '[]',
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_used_at    TEXT
);

CREATE INDEX idx_passkeys_user ON passkeys(user_id);

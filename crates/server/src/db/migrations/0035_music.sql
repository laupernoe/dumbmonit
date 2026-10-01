-- Musique du mode mur (`crates/server/src/music`).
--
-- `music_spotify` : le compte Spotify relié à l'instance, une seule ligne. Le
-- client ID est celui de l'application Spotify Developer que l'opérateur a créée
-- (flux Authorization Code + PKCE : pas de secret client). Le jeton de
-- rafraîchissement est chiffré avec le secret d'instance (`crypto.rs`) et ne
-- quitte jamais le serveur ; il vaut NULL quand Spotify l'a révoqué ou qu'il a
-- expiré (six mois après `authorized_at`), et `last_error` dit alors pourquoi.
-- Le jeton d'accès, valable une heure, ne vit qu'en mémoire.
CREATE TABLE music_spotify (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    client_id     TEXT    NOT NULL,
    refresh_token BLOB,
    scopes        TEXT    NOT NULL DEFAULT '',
    account_id    TEXT,
    account_name  TEXT,
    authorized_at TEXT    NOT NULL DEFAULT (datetime('now')),
    last_error    TEXT,
    updated_at    TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- `music_wall_link` : le lien Spotify, Deezer ou YouTube que les écrans muraux
-- jouent (« Play on the wall »), une seule ligne. Partagé par tous les murs :
-- on l'envoie depuis un téléphone, la télé le joue.
CREATE TABLE music_wall_link (
    id     INTEGER PRIMARY KEY CHECK (id = 1),
    link   TEXT    NOT NULL,
    set_by TEXT    NOT NULL,
    set_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Surveillance des changements d'un site web (`crates/server/src/webchange`).
--
-- `webchange_state` : une ligne par cible. `running_since` est le verrou qui
-- empêche deux vérifications simultanées (posé par un UPDATE conditionnel,
-- donc atomique ; tenu pour périmé au bout de trente minutes, si le serveur
-- s'est arrêté au milieu). `fingerprint` est l'empreinte des réglages qui
-- changent la comparaison (adresse, périmètre, exclusions) : NULL tant
-- qu'aucune référence n'a été prise, et une empreinte différente fait repartir
-- d'une référence neuve. `last_error_kind` : `down`, `auth` ou `config`.
CREATE TABLE webchange_state (
    target_id          INTEGER PRIMARY KEY REFERENCES targets(id) ON DELETE CASCADE,
    fingerprint        TEXT,
    running_since      TEXT,
    last_check_at      TEXT,
    last_check_changes INTEGER NOT NULL DEFAULT 0,
    last_error         TEXT,
    last_error_kind    TEXT
);

-- `webchange_pages` : chaque adresse vue, avec ce que la dernière lecture en a
-- dit. `removed` : la page a disparu (404/410, ou plus atteinte par un parcours
-- complet du site) ; elle reste listée, avec son dernier instantané.
CREATE TABLE webchange_pages (
    id              INTEGER PRIMARY KEY,
    target_id       INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    url             TEXT    NOT NULL,
    title           TEXT,
    status          INTEGER,
    error           TEXT,
    hash            TEXT,
    first_seen      TEXT    NOT NULL DEFAULT (datetime('now')),
    last_checked    TEXT,
    last_changed    TEXT,
    removed         INTEGER NOT NULL DEFAULT 0,
    latest_snapshot INTEGER,
    UNIQUE (target_id, url)
);

-- `webchange_snapshots` : le texte d'une page à un instant, enregistré
-- seulement quand il diffère du précédent. La capture d'écran éventuelle est
-- un fichier, `<data>/webchange/<target>/<id>.jpg` (voir `webchange::store`) ;
-- `has_screenshot` dit s'il existe. AUTOINCREMENT : un identifiant n'est
-- jamais réutilisé, donc un fichier orphelin ne peut jamais passer pour la
-- capture d'un instantané plus récent.
CREATE TABLE webchange_snapshots (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    target_id      INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    url            TEXT    NOT NULL,
    fetched_at     TEXT    NOT NULL DEFAULT (datetime('now')),
    title          TEXT,
    hash           TEXT    NOT NULL,
    content        TEXT    NOT NULL,
    has_screenshot INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX webchange_snapshots_target ON webchange_snapshots (target_id, url);

-- `webchange_changes` : les changements constatés, cent au plus par cible.
-- `added` et `removed` comptent des lignes de texte.
CREATE TABLE webchange_changes (
    id              INTEGER PRIMARY KEY,
    target_id       INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    url             TEXT    NOT NULL,
    kind            TEXT    NOT NULL CHECK (kind IN ('changed', 'new_page', 'removed_page')),
    detected_at     TEXT    NOT NULL DEFAULT (datetime('now')),
    added           INTEGER NOT NULL DEFAULT 0,
    removed         INTEGER NOT NULL DEFAULT 0,
    before_snapshot INTEGER,
    after_snapshot  INTEGER
);

CREATE INDEX webchange_changes_target ON webchange_changes (target_id, id);

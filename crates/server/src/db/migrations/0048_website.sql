-- Santé et référencement d'un site web (`crates/server/src/website`).
--
-- `website_state` : une ligne par cible. `tracker` est la référence confirmée
-- des métadonnées et les valeurs en attente de confirmation (JSON, voir
-- `website/changes.rs`). `fingerprint` est l'empreinte des réglages qui
-- changent la comparaison (adresse, `ignore_query`) : une empreinte
-- différente fait repartir d'une référence neuve. `changed_at` : champ ->
-- date du dernier changement confirmé (JSON). `last_probe` : le résultat de
-- la dernière lecture, pour l'API (JSON). `robots_*` / `sitemap_ok` : la
-- lecture de `robots.txt` et du plan de site, renouvelée toutes les heures.
CREATE TABLE website_state (
    target_id         INTEGER PRIMARY KEY REFERENCES targets(id) ON DELETE CASCADE,
    fingerprint       TEXT    NOT NULL,
    tracker           TEXT    NOT NULL,
    changed_at        TEXT    NOT NULL DEFAULT '{}',
    last_probe        TEXT,
    last_probe_at     TEXT,
    last_error        TEXT,
    robots_checked_at TEXT,
    robots_blocks_all INTEGER,
    sitemap_ok        INTEGER
);

-- `website_changes` : chaque changement confirmé d'un champ suivi.
CREATE TABLE website_changes (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    target_id   INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    detected_at TEXT    NOT NULL DEFAULT (datetime('now')),
    field       TEXT    NOT NULL,
    before      TEXT    NOT NULL,
    after       TEXT    NOT NULL
);
CREATE INDEX website_changes_target ON website_changes (target_id, id);

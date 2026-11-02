-- Changements de configuration à faire après une mise à jour (`upgrade_notes`) :
-- une ligne par note que l'administrateur a fermée, par version du catalogue.
-- Fermée, une note ne revient plus jamais pour cette version. Les versions
-- vues (`upgrade_notes_seen`, `upgrade_notes_from`) et l'annonce faite dans la
-- fenêtre (`upgrade_notes_announced`) vivent dans `app_settings`.
CREATE TABLE upgrade_notes_closed (
    version   TEXT NOT NULL,
    note_id   TEXT NOT NULL,
    closed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
    closed_by TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (version, note_id)
);

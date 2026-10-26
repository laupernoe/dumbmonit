-- Procédures (runbooks) : un texte Markdown qui dit quoi faire quand une alerte
-- sonne, rattaché à une règle, à un dossier ou à un type d'équipement.
--
-- Le texte est stocké tel quel et rendu par l'interface, qui échappe tout HTML :
-- rien de ce qui est saisi ici n'est jamais interprété comme du balisage.
CREATE TABLE runbooks (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT    NOT NULL,
    body       TEXT    NOT NULL DEFAULT '',
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now')),
    updated_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now')),
    updated_by TEXT
);

-- Liaisons : une procédure peut couvrir plusieurs règles, dossiers ou types.
--   scope = 'rule'  : `value` est l'uid de la règle (stable d'une instance à l'autre) ;
--   scope = 'group' : `value` est le nom du dossier (`targets.group_name`) ;
--   scope = 'kind'  : `value` est le type d'équipement (`targets.kind`).
CREATE TABLE runbook_links (
    runbook_id INTEGER NOT NULL REFERENCES runbooks(id) ON DELETE CASCADE,
    scope      TEXT    NOT NULL CHECK (scope IN ('rule', 'group', 'kind')),
    value      TEXT    NOT NULL,
    PRIMARY KEY (runbook_id, scope, value)
);

CREATE INDEX idx_runbook_links_scope ON runbook_links(scope, value);

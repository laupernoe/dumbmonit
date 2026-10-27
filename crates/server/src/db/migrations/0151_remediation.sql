-- Remédiation : actions typées, politiques de déclenchement automatique et
-- journal d'audit de chaque exécution.
--
-- Le catalogue est fermé : `kind` ne prend que les valeurs connues du serveur,
-- et aucun champ ne porte de commande shell. Un script ne se désigne que par
-- son nom ; c'est l'agent qui sait, chez lui, ce que ce nom exécute.
CREATE TABLE remediation_actions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL UNIQUE,
    kind        TEXT    NOT NULL
                CHECK (kind IN ('service.restart', 'container.restart', 'script.run', 'webhook')),
    -- Paramètres non secrets, en JSON, validés par le schéma du type.
    params      TEXT    NOT NULL DEFAULT '{}',
    -- Paramètres secrets (jeton d'un webhook), chiffrés par `crypto.rs`. Jamais
    -- renvoyés par l'API, qui ne dit que leur présence.
    secrets     BLOB,
    -- Équipement visé d'office. NULL : celui de l'alerte qui déclenche.
    target_id   INTEGER REFERENCES targets(id) ON DELETE CASCADE,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now'))
);

-- Déclenchement automatique d'une action sur une règle.
--
-- Une politique naît désarmée (`enabled = 0`). L'armer se fait en deux temps :
-- `arm` remet un jeton à usage unique, de courte durée, que seul le même compte
-- peut présenter à `confirm`. Toute modification de la politique ou de son
-- action la désarme. Deux échecs de suite l'arrêtent (`halted_at`).
CREATE TABLE remediation_policies (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    action_id          INTEGER NOT NULL REFERENCES remediation_actions(id) ON DELETE CASCADE,
    rule_uid           TEXT    NOT NULL,
    -- Restreint la politique à un équipement ; NULL : tout équipement de la règle.
    target_id          INTEGER REFERENCES targets(id) ON DELETE CASCADE,
    after_minutes      INTEGER NOT NULL DEFAULT 5  CHECK (after_minutes BETWEEN 1 AND 1440),
    max_attempts       INTEGER NOT NULL DEFAULT 1  CHECK (max_attempts BETWEEN 1 AND 10),
    window_minutes     INTEGER NOT NULL DEFAULT 60 CHECK (window_minutes BETWEEN 5 AND 10080),
    enabled            INTEGER NOT NULL DEFAULT 0  CHECK (enabled IN (0, 1)),
    armed_by           TEXT,
    armed_at           TEXT,
    confirm_hash       TEXT,
    confirm_user       TEXT,
    confirm_expires_at TEXT,
    halted_at          TEXT,
    halt_reason        TEXT,
    created_at         TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now')),
    updated_at         TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now'))
);

CREATE INDEX idx_remediation_policies_rule ON remediation_policies(rule_uid);

-- Journal d'audit : qui, quoi, quand, sur quoi, avec quel résultat. Les
-- paramètres sont recopiés sans leurs secrets ; la sortie est tronquée. Les
-- noms sont recopiés aussi : le journal survit à la suppression d'une action.
CREATE TABLE remediation_runs (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    action_id        INTEGER REFERENCES remediation_actions(id) ON DELETE SET NULL,
    action_name      TEXT    NOT NULL,
    kind             TEXT    NOT NULL,
    params           TEXT    NOT NULL DEFAULT '{}',
    target_id        INTEGER,
    target_name      TEXT,
    fingerprint      TEXT,
    rule_uid         TEXT,
    policy_id        INTEGER,
    -- `manual` (un compte) ou `auto` (une politique).
    origin           TEXT    NOT NULL CHECK (origin IN ('manual', 'auto')),
    requested_by     TEXT    NOT NULL,
    status           TEXT    NOT NULL DEFAULT 'queued'
                     CHECK (status IN ('queued', 'running', 'done', 'failed', 'cancelled', 'expired')),
    output           TEXT,
    agent_command_id INTEGER,
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S', 'now')),
    finished_at      TEXT
);

CREATE INDEX idx_remediation_runs_created ON remediation_runs(created_at);
CREATE INDEX idx_remediation_runs_target ON remediation_runs(target_id, id);
CREATE INDEX idx_remediation_runs_fingerprint ON remediation_runs(fingerprint, id);
CREATE INDEX idx_remediation_runs_policy ON remediation_runs(policy_id, id);
CREATE INDEX idx_remediation_runs_command ON remediation_runs(agent_command_id);

-- Ce que l'agent déclare savoir faire : actions activées ou non, services
-- autorisés, scripts présents dans son dossier local (JSON). NULL : un agent
-- antérieur, qui n'exécutera ni service ni script.
ALTER TABLE agent_hosts ADD COLUMN remediation TEXT;

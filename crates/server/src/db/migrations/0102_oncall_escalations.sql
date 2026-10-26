-- Astreintes : politiques d'escalade en plusieurs étapes et leur journal.
--
-- Comme un planning, une politique est un document JSON (étapes, cibles,
-- délais, répétitions) lu par `oncall::escalation`.
CREATE TABLE oncall_policies (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    description TEXT    NOT NULL DEFAULT '',
    definition  TEXT    NOT NULL DEFAULT '{}',
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Une escalade en cours ou terminée : une alerte (empreinte + épisode) prise
-- en charge par un canal d'astreinte. `item` garde de quoi rédiger les appels
-- suivants sans relire tout le cycle.
CREATE TABLE oncall_escalations (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    policy_id   INTEGER NOT NULL,
    channel_id  INTEGER NOT NULL,
    fingerprint TEXT    NOT NULL,
    episode     TEXT    NOT NULL DEFAULT '',
    target_id   INTEGER,
    target_name TEXT    NOT NULL DEFAULT '',
    rule_name   TEXT    NOT NULL DEFAULT '',
    severity    TEXT    NOT NULL DEFAULT 'warning',
    item        TEXT    NOT NULL DEFAULT '{}',
    status      TEXT    NOT NULL DEFAULT 'active',
    next_step   INTEGER NOT NULL DEFAULT 0,
    round       INTEGER NOT NULL DEFAULT 0,
    next_at     TEXT,
    acked_by    TEXT,
    started_at  TEXT    NOT NULL,
    ended_at    TEXT
);
CREATE INDEX idx_oncall_escalations_status ON oncall_escalations(status, next_at);
CREATE INDEX idx_oncall_escalations_fingerprint ON oncall_escalations(fingerprint, channel_id);

-- Qui a été appelé, quand, par quel moyen, et qui a acquitté.
CREATE TABLE oncall_escalation_events (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    escalation_id INTEGER NOT NULL REFERENCES oncall_escalations(id) ON DELETE CASCADE,
    at            TEXT    NOT NULL,
    step          INTEGER,
    round         INTEGER,
    event         TEXT    NOT NULL,
    user_id       INTEGER,
    user_name     TEXT,
    via           TEXT    NOT NULL DEFAULT '',
    kind          TEXT    NOT NULL DEFAULT '',
    detail        TEXT    NOT NULL DEFAULT '',
    -- Contact ou canal appelé avec succès, pour prévenir de la résolution.
    contact_id    INTEGER,
    channel_id    INTEGER
);
CREATE INDEX idx_oncall_events_escalation ON oncall_escalation_events(escalation_id, at);

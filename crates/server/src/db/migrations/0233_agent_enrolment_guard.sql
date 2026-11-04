-- Garde de l'enrôlement des agents (voir docs/install/agent-security.md).
--
-- Un jeton d'enrôlement ne sert qu'une fois par machine : ensuite l'agent a
-- son propre certificat client. Un jeton de flotte qui fuit permettrait donc
-- d'inscrire de faux agents ; ces tables protègent l'enrôlement lui-même.

-- Machines qui ont demandé à entrer et attendent (ou ont reçu) une décision.
--
-- Une ligne par clé d'identité : une machine qui réessaie toutes les minutes
-- ne remplit pas la liste. `status` : `pending` (en attente), `approved`
-- (acceptée, elle entrera à sa prochaine tentative), `rejected` (refusée ; elle
-- n'est plus montrée pendant 24 h, sauf si elle revient avec un autre jeton).
-- Rien n'est créé dans `targets` tant qu'elle n'est pas acceptée.
CREATE TABLE agent_enrolment_requests (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_key    TEXT    NOT NULL UNIQUE,
    hostname     TEXT    NOT NULL,
    os           TEXT    NOT NULL DEFAULT '',
    source_ip    TEXT,
    token_id     INTEGER REFERENCES agent_tokens(id) ON DELETE CASCADE,
    status       TEXT    NOT NULL DEFAULT 'pending'
                 CHECK (status IN ('pending', 'approved', 'rejected')),
    requested_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_seen_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    decided_at   TEXT,
    decided_by   TEXT
);

CREATE INDEX idx_agent_enrolment_requests_status ON agent_enrolment_requests(status);

-- Journal court des enrôlements : il sert au débit par jeton (une heure
-- glissante) et au message groupé envoyé chaque minute. Purgé après deux jours.
--
-- `kind` : `enrolled` (machine entrée), `pending` (nouvelle demande en
-- attente), `burst` (débit du jeton dépassé), `dormant` (jeton mis en veille).
CREATE TABLE agent_enrolment_events (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    token_id  INTEGER REFERENCES agent_tokens(id) ON DELETE CASCADE,
    hostname  TEXT    NOT NULL DEFAULT '',
    kind      TEXT    NOT NULL CHECK (kind IN ('enrolled', 'pending', 'burst', 'dormant')),
    at_ms     INTEGER NOT NULL,
    notified  INTEGER NOT NULL DEFAULT 0 CHECK (notified IN (0, 1))
);

CREATE INDEX idx_agent_enrolment_events_token ON agent_enrolment_events(token_id, at_ms);
CREATE INDEX idx_agent_enrolment_events_notified ON agent_enrolment_events(notified);

-- Mise en veille d'un jeton de flotte resté sans enrôlement trop longtemps.
--
-- `enrol_activity_at` : dernier enrôlement servi, ou dernière remise en
-- service. Les jetons existants partent de maintenant : une mise à jour du
-- serveur ne doit pas endormir d'un coup un jeton qu'on comptait utiliser.
-- `dormant_at` : moment de la mise en veille ; NULL tant que le jeton est en
-- service. Un jeton en veille refuse les enrôlements, les machines déjà
-- enrôlées continuent de remonter.
ALTER TABLE agent_tokens ADD COLUMN enrol_activity_at TEXT;
ALTER TABLE agent_tokens ADD COLUMN dormant_at TEXT;
UPDATE agent_tokens SET enrol_activity_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now');

-- Une instance qui a déjà des jetons garde le comportement qu'elle connaît :
-- acceptation automatique. Une base neuve n'écrit rien ici et prend les
-- valeurs par défaut (validation pour les jetons de flotte).
INSERT INTO settings (key, value)
SELECT 'agent.enrolment',
       CAST('{"fleet_approval":"auto","single_use_approval":"auto","rate_per_hour":60,"dormant_days":30}' AS BLOB)
WHERE EXISTS (SELECT 1 FROM agent_tokens)
  AND NOT EXISTS (SELECT 1 FROM settings WHERE key = 'agent.enrolment');

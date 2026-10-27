-- Journal d'événements Windows lu par l'agent.
--
-- Une machine sous Windows reçoit ses règles dans l'accusé de réception de
-- chaque lot ; l'agent interroge le journal avec elles et renvoie les
-- événements qui y répondent. Aucune règle n'existe par défaut : l'interface
-- propose des exemples (4625, 41 Kernel-Power, 7034) à ajouter.
CREATE TABLE agent_event_rules (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    -- La machine (cible `agent`) dont l'agent lit le journal.
    target_id      INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    name           TEXT    NOT NULL,
    -- `System`, `Application`, `Security` ou un canal (`…/Operational`).
    channel        TEXT    NOT NULL,
    -- Fournisseur ; NULL : tous.
    provider       TEXT,
    -- Forme canonique (« 4625, 7000-7099 ») ; vide : tous les identifiants.
    event_ids      TEXT    NOT NULL DEFAULT '',
    -- Niveaux séparés par des virgules (`critical,error`) ; vide : tous.
    levels         TEXT    NOT NULL DEFAULT '',
    -- Motif cherché dans le message ; NULL : aucun.
    pattern        TEXT,
    pattern_regex  INTEGER NOT NULL DEFAULT 0 CHECK (pattern_regex IN (0, 1)),
    severity       TEXT    NOT NULL DEFAULT 'warning'
                   CHECK (severity IN ('info', 'warning', 'critical')),
    enabled        INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    -- Événements reçus, et écartés par la limite de débit de l'agent.
    matches_total  INTEGER NOT NULL DEFAULT 0,
    dropped_total  INTEGER NOT NULL DEFAULT 0,
    -- Dernier événement reçu (ms depuis l'époque).
    last_match_ms  INTEGER,
    created_at     TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at     TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_agent_event_rules_target ON agent_event_rules(target_id);

-- Les derniers événements reçus, par machine (quelques centaines au plus :
-- l'ingestion élague). Le nom et la sévérité de la règle sont recopiés pour
-- que l'historique survive à sa suppression.
CREATE TABLE agent_event_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    target_id       INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    rule_id         INTEGER REFERENCES agent_event_rules(id) ON DELETE SET NULL,
    rule_name       TEXT    NOT NULL,
    severity        TEXT    NOT NULL,
    channel         TEXT    NOT NULL,
    provider        TEXT    NOT NULL DEFAULT '',
    event_id        INTEGER NOT NULL,
    level           INTEGER NOT NULL DEFAULT 0,
    record_id       INTEGER NOT NULL DEFAULT 0,
    computer        TEXT    NOT NULL DEFAULT '',
    message         TEXT    NOT NULL DEFAULT '',
    occurred_ms     INTEGER NOT NULL,
    received_ms     INTEGER NOT NULL
);

CREATE INDEX idx_agent_event_log_target ON agent_event_log(target_id, received_ms);

-- Ce que l'alerte « Windows event » de chaque sévérité sait d'une machine :
-- le nombre d'événements des quinze dernières minutes (la valeur de la jauge
-- écrite au même moment), le fait le plus récent annoncé et son résumé.
CREATE TABLE agent_event_alerts (
    target_id      INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    severity       TEXT    NOT NULL,
    matches        INTEGER NOT NULL DEFAULT 0,
    -- Instant du dernier fait annoncé : avancé au plus toutes les dix minutes,
    -- ce qui regroupe une rafale en une notification.
    announced_ms   INTEGER,
    detail         TEXT,
    PRIMARY KEY (target_id, severity)
) WITHOUT ROWID;

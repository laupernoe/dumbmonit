-- Tickets ouverts automatiquement par les canaux Jira, GLPI, Zammad, GitHub et
-- GitLab : un ticket externe par alerte et par canal.
--
-- La clé est l'empreinte de l'alerte : tant qu'une ligne `open` existe, aucun
-- second ticket n'est créé pour la même alerte (dédoublonnage). Une ligne
-- `closed` est gardée pour pouvoir rouvrir le ticket si l'alerte revient dans
-- le délai choisi sur le canal. Supprimer le canal emporte ses liens ; les
-- tickets eux-mêmes restent dans l'outil qui les héberge.

CREATE TABLE alert_tickets (
    channel_id  INTEGER NOT NULL REFERENCES notification_channels(id) ON DELETE CASCADE,
    fingerprint TEXT    NOT NULL,
    -- Identifiant utilisé par l'API du service (clé Jira, numéro GLPI…).
    external_id TEXT    NOT NULL,
    -- Libellé affiché (« PROJ-12 », « #34 »).
    label       TEXT    NOT NULL,
    url         TEXT    NOT NULL,
    state       TEXT    NOT NULL CHECK (state IN ('open', 'closed')),
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL,
    closed_at   TEXT,
    PRIMARY KEY (channel_id, fingerprint)
);

CREATE INDEX idx_alert_tickets_fingerprint ON alert_tickets (fingerprint);

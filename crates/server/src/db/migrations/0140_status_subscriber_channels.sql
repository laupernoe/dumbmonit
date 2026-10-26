-- Abonnés des pages de statut par SMS, webhook signé ou Slack, en plus du
-- courriel. La colonne `email` reste la destination de l'abonné, quel que soit
-- son canal (adresse, numéro E.164 ou URL) : l'unicité (page, destination) et
-- le désabonnement par jeton ne changent pas.

-- Canaux ouverts par page : le fournisseur SMS est un canal de notification déjà
-- configuré (NULL : pas de SMS) ; webhook et Slack n'ont pas besoin de canal,
-- le visiteur donne sa propre URL.
ALTER TABLE status_pages ADD COLUMN subscribe_sms_channel_id INTEGER
    REFERENCES notification_channels(id) ON DELETE SET NULL;
ALTER TABLE status_pages ADD COLUMN subscribe_webhook INTEGER NOT NULL DEFAULT 0;
ALTER TABLE status_pages ADD COLUMN subscribe_slack INTEGER NOT NULL DEFAULT 0;

ALTER TABLE status_subscribers ADD COLUMN kind TEXT NOT NULL DEFAULT 'email'
    CHECK (kind IN ('email', 'sms', 'webhook', 'slack'));
-- Services suivis : tableau JSON d'identifiants d'équipement ; NULL : tous.
ALTER TABLE status_subscribers ADD COLUMN components TEXT;
-- Webhook : secret HMAC chiffré par la clé d'instance.
ALTER TABLE status_subscribers ADD COLUMN secret BLOB;
-- SMS : empreinte du code de confirmation et essais déjà faits.
ALTER TABLE status_subscribers ADD COLUMN code_hash TEXT;
ALTER TABLE status_subscribers ADD COLUMN code_attempts INTEGER NOT NULL DEFAULT 0;

-- Services touchés par une annonce (facultatif : aucun = toute la page).
CREATE TABLE incident_components (
    incident_id INTEGER NOT NULL REFERENCES incidents(id) ON DELETE CASCADE,
    target_id   INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    PRIMARY KEY (incident_id, target_id)
);

-- File d'envoi des annonces vers les abonnés SMS, webhook et Slack : une ligne
-- par abonné et par annonce, reprise avec un délai croissant en cas d'échec.
CREATE TABLE status_deliveries (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    subscriber_id   INTEGER NOT NULL REFERENCES status_subscribers(id) ON DELETE CASCADE,
    payload         TEXT    NOT NULL,
    attempts        INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX idx_status_deliveries_due ON status_deliveries(next_attempt_at);

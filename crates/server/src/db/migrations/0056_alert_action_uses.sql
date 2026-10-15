-- Liens d'action des notifications (`notify/action_link.rs`) : chaque lien ne
-- sert qu'une fois. On garde l'aléa du lien jusqu'à son échéance, après quoi
-- la signature suffit à le refuser et la ligne est purgée.
CREATE TABLE alert_action_uses (
    nonce      TEXT PRIMARY KEY,
    used_at    TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE INDEX alert_action_uses_expires ON alert_action_uses (expires_at);

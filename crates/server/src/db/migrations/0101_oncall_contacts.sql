-- Astreintes : moyens de contact et préférences de chaque compte.
--
-- Un contact est soit un service personnel décrit comme un canal (même type,
-- mêmes réglages, secrets chiffrés par le secret d'instance), soit une adresse
-- servie par un canal d'équipe existant (`via_channel_id`) : le serveur SMTP
-- ou le compte Twilio de l'équipe, avec le seul destinataire changé.
CREATE TABLE oncall_contacts (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id        INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    label          TEXT    NOT NULL,
    kind           TEXT    NOT NULL CHECK (kind <> ''),
    via_channel_id INTEGER REFERENCES notification_channels(id) ON DELETE CASCADE,
    address        TEXT,
    settings       TEXT    NOT NULL DEFAULT '{}',
    secret_enc     BLOB,
    enabled        INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    min_severity   TEXT    NOT NULL DEFAULT 'info',
    position       INTEGER NOT NULL DEFAULT 0,
    last_error     TEXT,
    last_used_at   TEXT,
    created_at     TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX idx_oncall_contacts_user ON oncall_contacts(user_id, position);

-- Préférences d'astreinte : fuseau, heures calmes (même forme que celles d'un
-- canal) et empreinte SHA-256 du jeton du lien iCal personnel. Le jeton
-- lui-même n'est montré qu'une fois, à sa création.
CREATE TABLE oncall_user_prefs (
    user_id         INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    timezone        TEXT    NOT NULL DEFAULT '',
    quiet_hours     TEXT,
    ical_token_hash TEXT    UNIQUE,
    updated_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

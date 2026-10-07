-- Rapports périodiques par courriel : disponibilité, incidents, équipements
-- instables. Une ligne = un calendrier d'envoi.
--
-- `armed_at` est le point de départ de l'anti-rattrapage : un créneau antérieur
-- à la création (ou à la dernière activation) n'est jamais envoyé après coup.
-- `last_sent_at` ferme la porte aux doublons : un créneau n'est réclamé qu'une
-- fois, par un UPDATE conditionnel, avant même l'envoi.
CREATE TABLE report_schedules (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT    NOT NULL,
    enabled      INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    frequency    TEXT    NOT NULL DEFAULT 'weekly' CHECK (frequency IN ('daily', 'weekly', 'monthly')),
    weekday      INTEGER NOT NULL DEFAULT 0 CHECK (weekday BETWEEN 0 AND 6),     -- 0 = lundi
    day_of_month INTEGER NOT NULL DEFAULT 1 CHECK (day_of_month BETWEEN 1 AND 28),
    hour         INTEGER NOT NULL DEFAULT 8 CHECK (hour BETWEEN 0 AND 23),
    timezone     TEXT    NOT NULL DEFAULT 'UTC',
    recipients   TEXT    NOT NULL DEFAULT '[]',                                   -- tableau JSON d'adresses
    channel_id   INTEGER REFERENCES notification_channels(id) ON DELETE SET NULL, -- NULL : premier canal SMTP actif
    armed_at     TEXT    NOT NULL,
    last_sent_at TEXT,
    last_error   TEXT,
    created_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);

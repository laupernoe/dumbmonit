-- no-transaction
--
-- Notifications Web Push : la paire de clés VAPID de l'instance et les
-- abonnements des navigateurs (un par appareil et par compte).
--
-- La clé privée VAPID et les clés de chiffrement de chaque abonnement sont
-- chiffrées par le secret d'instance (`crate::crypto::Cipher`) ; la clé publique
-- VAPID, elle, est donnée à tout navigateur qui s'abonne et reste en clair.
--
-- Au passage, `notification_channels` perd la liste figée de ses types : la
-- contrainte posée par 0003 n'admettait que les sept premiers services, si bien
-- qu'un canal Matrix, Teams, Pushover… (et désormais Web Push) échouait à
-- l'enregistrement. L'API valide le type contre `notify::CHANNEL_KINDS`, seule
-- source de vérité. SQLite ne sait pas retirer une contrainte : la table est
-- reconstruite selon la procédure de la documentation SQLite (clés étrangères
-- coupées le temps de l'échange, sans quoi supprimer l'ancienne table viderait
-- en cascade la file d'attente des notifications). D'où `no-transaction` en
-- tête : `PRAGMA foreign_keys` est sans effet à l'intérieur d'une transaction,
-- qui est donc ouverte ici, après lui.

PRAGMA foreign_keys = OFF;

BEGIN;

CREATE TABLE notification_channels_new (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT    NOT NULL UNIQUE,
    kind         TEXT    NOT NULL CHECK (kind <> ''),
    enabled      INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    settings     TEXT    NOT NULL DEFAULT '{}',
    secret_enc   BLOB,
    last_error   TEXT,
    last_sent_at TEXT,
    created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    policy       TEXT    NOT NULL DEFAULT '{}'
);

INSERT INTO notification_channels_new
    (id, name, kind, enabled, settings, secret_enc, last_error, last_sent_at,
     created_at, updated_at, policy)
SELECT id, name, kind, enabled, settings, secret_enc, last_error, last_sent_at,
       created_at, updated_at, policy
FROM notification_channels;

DROP TABLE notification_channels;
ALTER TABLE notification_channels_new RENAME TO notification_channels;

CREATE TABLE webpush_vapid (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    -- Point P-256 non compressé (65 octets), en base64url sans remplissage :
    -- exactement la forme qu'attend `pushManager.subscribe`.
    public_key      TEXT    NOT NULL,
    -- Scalaire privé (32 octets), chiffré.
    private_key_enc BLOB    NOT NULL,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE webpush_subscriptions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Adresse du service de push du navigateur. Unique : un navigateur n'a qu'un
    -- abonnement par portée, et le renouveler remplace la ligne existante.
    endpoint        TEXT    NOT NULL UNIQUE,
    -- Clé publique ECDH du navigateur (65 octets) et secret d'authentification
    -- (16 octets), chiffrés.
    p256dh_enc      BLOB    NOT NULL,
    auth_enc        BLOB    NOT NULL,
    -- Libellé lisible déduit du User-Agent (« Firefox on Android »).
    device          TEXT    NOT NULL DEFAULT '',
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    last_success_at TEXT,
    last_error      TEXT
);

CREATE INDEX idx_webpush_subscriptions_user ON webpush_subscriptions(user_id);

COMMIT;

PRAGMA foreign_keys = ON;

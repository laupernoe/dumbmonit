-- Mode multi-clients (MSP).
--
-- Un client (locataire) se place au-dessus des équipes et des dossiers : chaque
-- équipement, page de statut et jeton d'agent appartient à un client ; les
-- canaux de notification, règles d'alerte, silences et calendriers de rapport
-- appartiennent à un client ou, `client_id` NULL, à l'instance (objets
-- partagés par l'administrateur MSP).
--
-- Tout l'existant est rattaché au client par défaut (id 1) — ou laissé à
-- l'instance pour les objets partageables — et les comptes non
-- administrateurs en deviennent membres : tant que le mode MSP n'est pas
-- activé (réglage `msp`), rien ne change.

CREATE TABLE clients (
    id                 INTEGER PRIMARY KEY,
    name               TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    description        TEXT    NOT NULL DEFAULT '',
    -- Adresses des contacts du client (tableau JSON) : destinataires du
    -- rapport SLA mensuel automatique.
    contacts           TEXT    NOT NULL DEFAULT '[]',
    -- Nombre maximal d'équipements ; NULL : sans limite.
    monitor_quota      INTEGER CHECK (monitor_quota IS NULL OR monitor_quota >= 0),
    -- Groupes du fournisseur d'identité dont les membres rejoignent le client à
    -- la connexion (tableau JSON de chaînes).
    idp_groups         TEXT    NOT NULL DEFAULT '[]',
    -- Marque légère : logo (PNG, JPEG ou WebP, quelques centaines de Ko au
    -- plus), affiché sur les rapports et les pages de statut du client.
    logo               BLOB,
    logo_type          TEXT,
    -- Rapport SLA mensuel envoyé automatiquement aux contacts.
    sla_report         INTEGER NOT NULL DEFAULT 0 CHECK (sla_report IN (0, 1)),
    sla_objective      REAL,
    report_timezone    TEXT    NOT NULL DEFAULT 'UTC',
    -- Dernier mois servi (`AAAA-MM`) : un mois n'est jamais envoyé deux fois.
    report_last_period TEXT,
    report_last_error  TEXT,
    created_at         TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
    updated_at         TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now'))
);

INSERT INTO clients (id, name) VALUES (1, 'Default');

-- `source` : comme pour les équipes. `manual` n'est jamais retiré par une
-- synchronisation ; `idp` suit les groupes à chaque connexion.
CREATE TABLE client_members (
    client_id INTEGER NOT NULL REFERENCES clients(id) ON DELETE CASCADE,
    user_id   INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source    TEXT    NOT NULL DEFAULT 'manual' CHECK (source IN ('manual', 'idp', 'scim')),
    PRIMARY KEY (client_id, user_id)
);

CREATE INDEX idx_client_members_user ON client_members(user_id);

INSERT INTO client_members (client_id, user_id, source)
SELECT 1, id, 'manual' FROM users WHERE role <> 'admin';

-- Tant que le mode MSP est coupé, un compte non administrateur créé (ou
-- rétrogradé) rejoint le client par défaut : activer le mode plus tard ne
-- surprend personne. Mode activé, un nouveau compte n'appartient à aucun client
-- et ne voit rien tant qu'un administrateur ne l'a pas rattaché.
CREATE TRIGGER users_default_client AFTER INSERT ON users
WHEN NEW.role <> 'admin'
 AND COALESCE(json_extract(CAST((SELECT value FROM settings WHERE key = 'msp') AS TEXT),
                           '$.enabled'), 0) = 0
BEGIN
    INSERT OR IGNORE INTO client_members (client_id, user_id, source) VALUES (1, NEW.id, 'manual');
END;

CREATE TRIGGER users_default_client_demoted AFTER UPDATE OF role ON users
WHEN OLD.role = 'admin' AND NEW.role <> 'admin'
 AND COALESCE(json_extract(CAST((SELECT value FROM settings WHERE key = 'msp') AS TEXT),
                           '$.enabled'), 0) = 0
 AND NOT EXISTS (SELECT 1 FROM client_members WHERE user_id = NEW.id)
BEGIN
    INSERT OR IGNORE INTO client_members (client_id, user_id, source) VALUES (1, NEW.id, 'manual');
END;

-- Équipements : toujours un client. SQLite refuse d'ajouter une colonne
-- `REFERENCES` avec une valeur par défaut non nulle : l'intégrité est tenue par
-- les déclencheurs ci-dessous.
ALTER TABLE targets ADD COLUMN client_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX idx_targets_client ON targets(client_id);

CREATE TRIGGER targets_client_exists_insert BEFORE INSERT ON targets
WHEN NOT EXISTS (SELECT 1 FROM clients WHERE id = NEW.client_id)
BEGIN
    SELECT RAISE(ABORT, 'unknown client');
END;

CREATE TRIGGER targets_client_exists_update BEFORE UPDATE OF client_id ON targets
WHEN NOT EXISTS (SELECT 1 FROM clients WHERE id = NEW.client_id)
BEGIN
    SELECT RAISE(ABORT, 'unknown client');
END;

CREATE TRIGGER clients_in_use BEFORE DELETE ON clients
WHEN EXISTS (SELECT 1 FROM targets WHERE client_id = OLD.id)
BEGIN
    SELECT RAISE(ABORT, 'client in use');
END;

-- Pages de statut et jetons d'agent : un client (NULL n'est accepté que pour
-- une page de l'instance). `RESTRICT` : un client qui en a encore ne se
-- supprime pas.
ALTER TABLE status_pages ADD COLUMN client_id INTEGER REFERENCES clients(id) ON DELETE RESTRICT;
UPDATE status_pages SET client_id = 1;

ALTER TABLE agent_tokens ADD COLUMN client_id INTEGER REFERENCES clients(id) ON DELETE RESTRICT;
UPDATE agent_tokens SET client_id = 1;

-- Objets partageables : NULL = l'instance. Supprimer le client supprime les
-- siens — les rendre à l'instance les ferait parler pour tous les clients.
ALTER TABLE notification_channels
    ADD COLUMN client_id INTEGER REFERENCES clients(id) ON DELETE CASCADE;
ALTER TABLE alert_rules ADD COLUMN client_id INTEGER REFERENCES clients(id) ON DELETE CASCADE;
ALTER TABLE silences ADD COLUMN client_id INTEGER REFERENCES clients(id) ON DELETE CASCADE;
ALTER TABLE report_schedules
    ADD COLUMN client_id INTEGER REFERENCES clients(id) ON DELETE CASCADE;

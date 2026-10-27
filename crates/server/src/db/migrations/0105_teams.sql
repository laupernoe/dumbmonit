-- Équipes et droits par dossier.
--
-- Une équipe regroupe des comptes et reçoit, dossier par dossier, un niveau de
-- droit : `read` (voir), `operate` (voir et traiter les alertes : acquitter,
-- mettre en sourdine, maintenance), `manage` (et modifier les équipements).
-- Le rôle global du compte reste le plafond (`viewer` → lecture, `operator` →
-- gestion) ; un administrateur n'est jamais restreint.
--
-- Un compte membre d'au moins une équipe ne voit plus que ce que ses équipes
-- lui ouvrent. Un compte sans équipe garde le comportement d'avant (tout, au
-- niveau de son rôle), sauf si le réglage `teams.access` dit `none`.

CREATE TABLE teams (
    id          INTEGER PRIMARY KEY,
    name        TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    description TEXT    NOT NULL DEFAULT '',
    -- Groupes du fournisseur d'identité (OIDC, LDAP, SAML) dont les membres
    -- rejoignent l'équipe à la connexion : tableau JSON de chaînes.
    idp_groups  TEXT    NOT NULL DEFAULT '[]',
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now'))
);

-- `source` : qui a inscrit le membre. `manual` (un administrateur) n'est jamais
-- retiré par une synchronisation ; `idp` suit les groupes à chaque connexion ;
-- `scim` suit le provisionnement.
CREATE TABLE team_members (
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source  TEXT    NOT NULL DEFAULT 'manual' CHECK (source IN ('manual', 'idp', 'scim')),
    PRIMARY KEY (team_id, user_id)
);

CREATE INDEX idx_team_members_user ON team_members(user_id);

-- `folder` : `*` pour tous les équipements, chaîne vide pour ceux qui ne sont
-- dans aucun dossier, sinon un dossier (`targets.group_name`). Un droit sur
-- `Paris` couvre aussi les sous-dossiers `Paris/…`.
CREATE TABLE team_folder_grants (
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    folder  TEXT    NOT NULL,
    level   TEXT    NOT NULL CHECK (level IN ('read', 'operate', 'manage')),
    PRIMARY KEY (team_id, folder)
);

-- Provisionnement SCIM 2.0 (Entra ID, Okta…).
--
-- Un fournisseur d'identité crée, met à jour, désactive et supprime des comptes
-- et des équipes par `/api/scim/v2`. Il ne voit que ce qu'il a provisionné ou
-- rattaché : les lignes dont `scim_id` est renseigné. Le jeton du fournisseur
-- n'est pas ici : seule son empreinte est gardée, dans `settings`.

-- Identifiant SCIM du compte (UUID, attribué par DumbMonit), identifiant du
-- compte chez le fournisseur (`externalId`), courriel annoncé, date de la
-- dernière écriture SCIM.
ALTER TABLE users ADD COLUMN scim_id TEXT;
ALTER TABLE users ADD COLUMN scim_external_id TEXT;
ALTER TABLE users ADD COLUMN scim_email TEXT;
ALTER TABLE users ADD COLUMN scim_updated_at TEXT;
-- Désactivé par le fournisseur (`active: false`), pas par un administrateur :
-- seul un compte que SCIM a désactivé peut être réactivé par SCIM.
ALTER TABLE users ADD COLUMN disabled_by_scim INTEGER NOT NULL DEFAULT 0;

CREATE UNIQUE INDEX idx_users_scim_id ON users(scim_id) WHERE scim_id IS NOT NULL;

-- Équipes : mêmes identifiants. `scim_created` distingue une équipe créée par
-- le fournisseur (sa suppression SCIM la supprime) d'une équipe existante qu'il
-- a seulement rattachée (sa suppression SCIM n'en retire que les membres SCIM).
ALTER TABLE teams ADD COLUMN scim_id TEXT;
ALTER TABLE teams ADD COLUMN scim_external_id TEXT;
ALTER TABLE teams ADD COLUMN scim_created INTEGER NOT NULL DEFAULT 0;
ALTER TABLE teams ADD COLUMN scim_updated_at TEXT;

CREATE UNIQUE INDEX idx_teams_scim_id ON teams(scim_id) WHERE scim_id IS NOT NULL;

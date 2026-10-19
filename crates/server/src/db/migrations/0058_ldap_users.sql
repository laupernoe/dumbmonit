-- Comptes provisionnés par l'annuaire LDAP / Active Directory.
--
-- `ldap_id` est l'identité stable de l'entrée dans l'annuaire : `objectGUID`
-- (AD) ou `entryUUID` (OpenLDAP), le DN à défaut. Un DN change quand
-- l'utilisateur est déplacé ou renommé ; l'identifiant, lui, reste.
ALTER TABLE users ADD COLUMN ldap_id TEXT;
-- Dernier DN connu, pour le diagnostic seulement.
ALTER TABLE users ADD COLUMN ldap_dn TEXT;
-- Désactivé par la connexion LDAP elle-même (l'utilisateur a perdu ses
-- groupes), et non par un administrateur : retrouver ses groupes le réactive,
-- alors qu'un compte désactivé à la main le reste.
ALTER TABLE users ADD COLUMN disabled_by_ldap INTEGER NOT NULL DEFAULT 0;

CREATE UNIQUE INDEX idx_users_ldap_id ON users(ldap_id) WHERE ldap_id IS NOT NULL;

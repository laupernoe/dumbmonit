-- Rôle `operator`, entre `viewer` et `admin` : il traite les alertes
-- (acquitter, ignorer, mettre en sourdine) sans toucher à la configuration.
--
-- SQLite ne modifie pas une contrainte CHECK en place, et la recréation
-- habituelle de la table est exclue ici : `users` est référencée par
-- `auth_sessions`, `api_tokens` et les codes de secours TOTP, et le DROP de
-- l'ancienne table, clés étrangères actives (elles le sont, et la migration
-- tourne dans une transaction où on ne peut pas les couper), viderait les
-- sessions en cascade et détacherait les jetons de leur propriétaire.
--
-- On réécrit donc le texte de la contrainte dans le schéma, comme la
-- documentation de SQLite l'admet pour une contrainte CHECK
-- (https://www.sqlite.org/lang_altertable.html, « otheralter ») : élargir la
-- liste des valeurs admises ne change rien au format des pages, et toutes les
-- lignes existantes la respectent déjà.
PRAGMA writable_schema = ON;

UPDATE sqlite_schema
SET sql = replace(
    sql,
    'CHECK (role IN (''admin'', ''viewer''))',
    'CHECK (role IN (''admin'', ''operator'', ''viewer''))'
)
WHERE type = 'table' AND name = 'users';

PRAGMA writable_schema = RESET;

-- Toute instruction DDL incrémente la version du schéma : les autres
-- connexions du pool relisent alors le schéma et voient la nouvelle
-- contrainte. L'index sert aussi le décompte des administrateurs actifs.
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);

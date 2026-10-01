-- Jetons d'API : expiration, restriction par réseau, adresse du dernier usage,
-- et révocation quand le compte qui les a créés disparaît.
--
-- Tout est facultatif et rétrocompatible : un jeton créé avant cette migration
-- n'expire pas, n'est restreint à aucun réseau et n'a pas de propriétaire
-- (`user_id` NULL) — il continue de fonctionner exactement comme avant.

-- Fin de validité, RFC 3339 UTC (`2026-12-31T10:00:00Z`). NULL : jamais.
ALTER TABLE api_tokens ADD COLUMN expires_at TEXT;

-- Réseaux d'où le jeton est accepté : tableau JSON de CIDR normalisés
-- (`["192.168.1.0/24","10.0.0.5/32"]`). NULL ou tableau vide : partout.
ALTER TABLE api_tokens ADD COLUMN allowed_networks TEXT;

-- Adresse du client lors du dernier usage, mise à jour avec `last_used_at`.
-- Un indice pour repérer un jeton qui sert d'où on ne l'attend pas, pas un
-- journal d'accès.
ALTER TABLE api_tokens ADD COLUMN last_used_ip TEXT;

-- Un jeton vaut ce que vaut le compte qui l'a créé. Supprimer le compte (ou
-- réinitialiser l'instance) révoque ses jetons : sans cela, `ON DELETE SET NULL`
-- en ferait des jetons sans propriétaire, que plus rien ne bride.
CREATE TRIGGER api_tokens_revoked_with_owner
BEFORE DELETE ON users
BEGIN
    UPDATE api_tokens
       SET revoked_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
     WHERE user_id = OLD.id AND revoked_at IS NULL;
END;

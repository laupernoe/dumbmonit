-- Jetons de flotte Active Directory.
--
-- `ad_domain` : domaine DNS (en minuscules) auquel le jeton est limité. Une
-- machine inconnue ne s'inscrit avec lui que si son nom complet se termine par
-- ce domaine ; elle est alors rangée d'office dans le dossier
-- « Active Directory <domaine> » avec l'étiquette `ad`. NULL : jeton ordinaire.
ALTER TABLE agent_tokens ADD COLUMN ad_domain TEXT;

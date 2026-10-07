-- Pages de statut : domaine public.
--
-- Quand une requête arrive avec ce nom dans l'en-tête `Host` (un mandataire
-- inverse qui pointe vers l'interface sans réécrire le chemin), le serveur sert
-- cette page à la racine et rien d'autre de l'instance (`crate::status_host`).
--
-- `domain` : nom d'hôte en minuscules, sans schéma, port ni chemin, validé par
-- `status_host::normalise_domain` ; NULL : pas de domaine. Un même nom ne peut
-- désigner deux pages (plusieurs NULL restent permis).
ALTER TABLE status_pages ADD COLUMN domain TEXT;
CREATE UNIQUE INDEX status_pages_domain ON status_pages (domain);

-- Mise à jour automatique de l'agent lui-même (pas des conteneurs Docker).
--
-- Ce que l'agent dit de son binaire à chaque lot, les réglages propres à la
-- machine, la demande explicite (« Update now »), la dernière offre faite et
-- l'issue de la dernière tentative telle que l'agent l'a rapportée. Les
-- réglages globaux vivent dans `app_settings` (`agent_update_*`).

-- SHA-256 du binaire qui tourne, et depuis quand (déploiement progressif).
ALTER TABLE agent_hosts ADD COLUMN agent_sha256 TEXT;
ALTER TABLE agent_hosts ADD COLUMN agent_sha_since_ms INTEGER;
-- `linux-x86_64`, `windows-x86_64`… : le binaire à proposer.
ALTER TABLE agent_hosts ADD COLUMN agent_platform TEXT;
-- `ready`, `container`, `read_only`, `disabled`, `unsupported` ; NULL : agent
-- antérieur à la mise à jour automatique.
ALTER TABLE agent_hosts ADD COLUMN update_ability TEXT;
-- `on` / `off` : forcé pour la machine ; NULL : le réglage global.
ALTER TABLE agent_hosts ADD COLUMN update_mode TEXT;
-- Machine pilote : mise à jour avant les autres.
ALTER TABLE agent_hosts ADD COLUMN update_canary INTEGER NOT NULL DEFAULT 0;
-- « Update now » : hors fenêtre, hors déploiement progressif, une nouvelle
-- tentative d'une version en échec.
ALTER TABLE agent_hosts ADD COLUMN update_requested_ms INTEGER;
ALTER TABLE agent_hosts ADD COLUMN update_allow_downgrade INTEGER NOT NULL DEFAULT 0;
-- Première offre du binaire `update_offered_sha`.
ALTER TABLE agent_hosts ADD COLUMN update_offered_ms INTEGER;
ALTER TABLE agent_hosts ADD COLUMN update_offered_sha TEXT;
-- Dernière tentative rapportée par l'agent.
ALTER TABLE agent_hosts ADD COLUMN update_outcome TEXT;
ALTER TABLE agent_hosts ADD COLUMN update_outcome_sha TEXT;
ALTER TABLE agent_hosts ADD COLUMN update_outcome_reason TEXT;
ALTER TABLE agent_hosts ADD COLUMN update_outcome_ms INTEGER;
ALTER TABLE agent_hosts ADD COLUMN update_outcome_to TEXT;

-- Mode relais piloté depuis l'interface.
--
-- `relay_wanted` / `relay_wanted_site` : ce que l'interface demande pour cette
-- machine, transmis à l'agent dans chaque accusé de réception ; il l'applique à
-- chaud. `relay` / `site` restent ce que l'agent déclare réellement.
--
-- `relay_control` : l'agent sait suivre ce réglage (faux pour un binaire
-- antérieur, que l'interface dit trop ancien). `relay_local` : `relay: true` est
-- posé dans sa configuration locale, qui prime.
--
-- Un jeton peut porter le mode relais (`agent_tokens.relay`, `relay_site`) : la
-- machine qu'il enrôle devient relais dès son premier lot, sans rien éditer.
ALTER TABLE agent_hosts ADD COLUMN relay_wanted INTEGER NOT NULL DEFAULT 0;
ALTER TABLE agent_hosts ADD COLUMN relay_wanted_site TEXT;
ALTER TABLE agent_hosts ADD COLUMN relay_control INTEGER NOT NULL DEFAULT 0;
ALTER TABLE agent_hosts ADD COLUMN relay_local INTEGER NOT NULL DEFAULT 0;

-- Une machine déjà relais par sa configuration le reste aussi côté interface.
UPDATE agent_hosts SET relay_wanted = relay, relay_wanted_site = site WHERE relay = 1;

ALTER TABLE agent_tokens ADD COLUMN relay INTEGER NOT NULL DEFAULT 0;
ALTER TABLE agent_tokens ADD COLUMN relay_site TEXT;

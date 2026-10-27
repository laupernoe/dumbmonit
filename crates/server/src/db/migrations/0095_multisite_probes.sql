-- Sondes depuis plusieurs emplacements : le serveur et/ou des agents relais
-- sondent le même moniteur, et une décision de quorum dit s'il est en panne.
--
-- Une ligne dans `target_multisite` active le mode pour la cible ; ses agents
-- sont dans `target_multisite_agents`. Supprimer la cible ou l'agent efface ce
-- qui le concerne.
CREATE TABLE target_multisite (
    target_id INTEGER PRIMARY KEY REFERENCES targets(id) ON DELETE CASCADE,
    -- Le serveur lui-même fait partie des emplacements.
    include_server INTEGER NOT NULL DEFAULT 1,
    -- Emplacements en échec qu'il faut pour déclarer la panne ; NULL : la majorité.
    quorum INTEGER CHECK (quorum IS NULL OR quorum >= 1)
);

CREATE TABLE target_multisite_agents (
    target_id INTEGER NOT NULL REFERENCES target_multisite(target_id) ON DELETE CASCADE,
    agent_id INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    PRIMARY KEY (target_id, agent_id)
);

CREATE INDEX target_multisite_agents_agent ON target_multisite_agents(agent_id);

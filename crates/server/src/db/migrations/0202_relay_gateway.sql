-- Passerelle des agents relais : les agents d'un site distant, derrière un
-- pare-feu, joignent le serveur à travers le relais du site, chiffrés de bout
-- en bout (le relais ne fait que transporter le TLS de l'agent).
--
-- `relay_gateway_wanted` : port sur lequel l'interface demande au relais
-- d'écouter (NULL : pas de passerelle) ; transmis dans l'accusé de réception.
-- `relay_gateway_port` : port sur lequel le relais déclare écouter vraiment,
-- à son dernier lot (NULL : pas de passerelle ouverte).
-- `relay_gateway_host` : adresse par laquelle les machines du site joignent le
-- relais (IP ou nom local), écrite dans les commandes d'installation.
ALTER TABLE agent_hosts ADD COLUMN relay_gateway_wanted INTEGER;
ALTER TABLE agent_hosts ADD COLUMN relay_gateway_port INTEGER;
ALTER TABLE agent_hosts ADD COLUMN relay_gateway_host TEXT;

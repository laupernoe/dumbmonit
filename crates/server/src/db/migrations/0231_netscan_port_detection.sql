-- Détection gardée par port de l'inventaire (`netinventory/`) : le type de
-- moniteur que le moteur de découverte propose (`kind`) et les champs de
-- formulaire pré-remplis (`prefill`, JSON sans secret). Ils permettent d'ajouter
-- en bloc des appareils « à examiner » sans repasser par un balayage.
ALTER TABLE netscan_ports ADD COLUMN kind TEXT;
ALTER TABLE netscan_ports ADD COLUMN prefill TEXT;

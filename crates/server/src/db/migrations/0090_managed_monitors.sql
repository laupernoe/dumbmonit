-- Moniteurs gérés par des étiquettes Docker (`dumbmonit.*`).
--
-- Un moniteur « géré » est créé, mis à jour et archivé par le serveur d'après les
-- étiquettes des conteneurs d'un agent : il n'est modifiable que par elles, tant
-- qu'on ne l'a pas libéré. `managed_key` est stable d'un cycle à l'autre
-- (`projet/service/http`), ce que l'identifiant d'un conteneur ne serait pas.
ALTER TABLE targets ADD COLUMN managed_by TEXT;
ALTER TABLE targets ADD COLUMN managed_source INTEGER REFERENCES targets(id) ON DELETE SET NULL;
ALTER TABLE targets ADD COLUMN managed_key TEXT;
-- Posé quand les étiquettes ne déclarent plus le moniteur : il est alors désactivé
-- (archivé) mais gardé avec son historique, et se réactive s'il revient.
ALTER TABLE targets ADD COLUMN managed_missing_since TEXT;

CREATE UNIQUE INDEX idx_targets_managed
    ON targets(managed_source, managed_key)
    WHERE managed_by IS NOT NULL;

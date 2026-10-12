-- Cadence de la comparaison des métadonnées d'un site (`website`).
--
-- La disponibilité est lue à chaque sonde, les métadonnées ne sont comparées
-- qu'aux sondes de changement (`change_check_minutes`, six heures par
-- défaut). `changes_checked_at` est la date de la dernière comparaison ;
-- vide, la prochaine sonde compare.
ALTER TABLE website_state ADD COLUMN changes_checked_at TEXT;

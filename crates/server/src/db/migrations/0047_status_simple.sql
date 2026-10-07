-- Pages de statut : mode simple et décor par défaut.
--
-- `simple` : 1 = page sobre (ni scène, ni pigeon, ni animation), thème clair ou
-- sombre selon le système du visiteur.
ALTER TABLE status_pages ADD COLUMN simple INTEGER NOT NULL DEFAULT 0;
-- Une page sans décor choisi montre désormais la scène par défaut
-- (`db::status_pages::DEFAULT_SCENE`) : on l'inscrit pour les pages existantes.
UPDATE status_pages SET scenes = 'paris' WHERE scenes = '';

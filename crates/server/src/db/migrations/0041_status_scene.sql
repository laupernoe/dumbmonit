-- Pages de statut : décor de ville en bannière.
--
-- Comme l'accent, un réglage d'un jeu fermé (jamais d'image ni de code libres) :
-- la liste des identifiants de scène est validée par `api::status_pages`.
--
-- `scenes` : identifiants séparés par des virgules, dans l'ordre d'affichage.
-- Vide : aucune scène.
ALTER TABLE status_pages ADD COLUMN scenes TEXT NOT NULL DEFAULT '';
-- `scene_rotation` : rythme de changement quand il y a plusieurs scènes
-- (`visit`, `1m`, `10m` ou `1h`).
ALTER TABLE status_pages ADD COLUMN scene_rotation TEXT NOT NULL DEFAULT 'visit';

-- Dossiers plats et ordre manuel des cibles, pour la page /targets.
--
-- `group_name` : un nom de dossier libre, non imbriqué — même vocabulaire que
-- `status_page_items.group_name`, qui jouait déjà ce rôle pour un usage voisin
-- (regroupement visuel d'une page de statut), plutôt que d'inventer un second
-- mot pour la même idée. Vide signifie « sans dossier » ; ces cibles-là restent
-- en tête de la page, hors de tout dossier.
--
-- `position` : rang manuel, départagé par `rack.ts` seulement entre cibles de
-- même état et (pour les cibles de premier niveau) du même dossier — un
-- équipement en panne reste toujours en tête, quel que soit l'ordre choisi.
-- Attribué à la création comme « après la dernière cible insérée », pour qu'une
-- nouvelle cible n'apparaisse jamais au hasard au milieu d'un ordre déjà trié
-- à la main.
ALTER TABLE targets ADD COLUMN group_name TEXT    NOT NULL DEFAULT '';
ALTER TABLE targets ADD COLUMN position   INTEGER NOT NULL DEFAULT 0;

CREATE INDEX idx_targets_group ON targets(group_name);

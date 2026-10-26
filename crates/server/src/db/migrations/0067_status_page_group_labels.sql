-- Pages de statut : nom public d'une surveillance arrivée par un groupe coché.
--
-- Les surveillances choisies une à une portent leur nom public dans
-- `status_page_items.label`. Celles qui arrivent par un groupe dynamique n'ont
-- pas de ligne là (elles disparaîtraient avec le groupe) : leur nom public,
-- facultatif, est gardé ici. Absent ou vide : le nom de la surveillance.
CREATE TABLE status_page_group_labels (
    page_id   INTEGER NOT NULL REFERENCES status_pages(id) ON DELETE CASCADE,
    target_id INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    label     TEXT    NOT NULL,
    PRIMARY KEY (page_id, target_id)
);

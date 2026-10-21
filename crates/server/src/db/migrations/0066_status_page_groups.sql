-- Pages de statut : groupes de surveillances cochés en bloc.
--
-- Un groupe coché est dynamique : les surveillances dont `targets.group_name`
-- vaut `group_name` apparaissent sur la page, celles qui le quittent
-- disparaissent. La liaison se fait par nom (les dossiers n'ont pas d'identifiant).
CREATE TABLE status_page_groups (
    page_id    INTEGER NOT NULL REFERENCES status_pages(id) ON DELETE CASCADE,
    group_name TEXT    NOT NULL,
    position   INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (page_id, group_name)
);

-- Tableaux de bord personnalisés : une grille de widgets (graphe, valeur,
-- état d'un dossier, alertes, texte) par tableau.
--
-- Un tableau est soit partagé (visible de tous, modifié par les
-- administrateurs), soit personnel (visible et modifiable par son seul
-- propriétaire). La grille est un document JSON validé par le serveur
-- (`api::dashboards`) : l'ajouter ou changer un type de widget ne demande pas
-- de migration.
CREATE TABLE dashboards (
    id           INTEGER PRIMARY KEY,
    name         TEXT    NOT NULL,
    shared       INTEGER NOT NULL DEFAULT 0,
    owner_id     INTEGER REFERENCES users(id) ON DELETE SET NULL,
    range_secs   INTEGER NOT NULL DEFAULT 3600,
    refresh_secs INTEGER NOT NULL DEFAULT 30,
    widgets      TEXT    NOT NULL DEFAULT '[]',
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now')),
    updated_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now'))
);

-- Un nom par propriétaire pour les tableaux personnels, un nom pour tous les
-- tableaux partagés.
CREATE UNIQUE INDEX idx_dashboards_name
    ON dashboards(shared, CASE WHEN shared = 1 THEN 0 ELSE owner_id END, name COLLATE NOCASE);

-- Les tableaux personnels d'un compte supprimé partent avec lui ; les
-- partagés restent (leur propriétaire devient nul).
CREATE TRIGGER dashboards_owner_deleted BEFORE DELETE ON users
BEGIN
    DELETE FROM dashboards WHERE shared = 0 AND owner_id = OLD.id;
END;

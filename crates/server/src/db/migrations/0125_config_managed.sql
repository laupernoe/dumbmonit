-- Objets déclarés par un fichier de configuration (GitOps).
--
-- Une règle, un canal, une fenêtre de maintenance, une page de statut ou une
-- surcharge déclarés par `POST /api/config/apply` (ou par le dossier surveillé
-- `DUMBMONIT_CONFIG_DIR`) sont inscrits ici : l'interface les affiche « gérés
-- par fichier », l'API refuse de les modifier à la main, et `prune` ne supprime
-- que ce qui figure dans cette table. Les équipements, eux, réutilisent
-- `targets.managed_by` (valeur `config-file`), comme les moniteurs nés des
-- étiquettes Docker.
--
-- `kind` : rule | override | channel | silence | status_page.
-- `key`  : clé naturelle de la sauvegarde (uid, `uid|équipement`, nom,
--          `nom` ou `nom|équipement`, slug).
CREATE TABLE config_managed (
    kind TEXT NOT NULL,
    key  TEXT NOT NULL,
    PRIMARY KEY (kind, key)
) WITHOUT ROWID;

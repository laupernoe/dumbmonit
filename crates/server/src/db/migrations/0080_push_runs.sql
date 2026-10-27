-- Exécutions des moniteurs en poussée : chaque `/start`, `/success` et `/fail`
-- d'un travail planifié laisse une ligne (début, fin, durée, code de sortie et
-- sortie tronquée). L'historique sert à la page du moniteur et à l'alerte
-- « échec », qui cite la sortie.
--
-- Les dates sont en millisecondes depuis l'époque Unix. `started_ms` est NULL
-- quand l'appel de fin n'a été précédé d'aucun `/start` (durée inconnue) ;
-- `finished_ms` est NULL tant que l'exécution tourne, ou si elle a été
-- remplacée par un nouveau `/start` avant d'avoir fini (statut `lost`).
CREATE TABLE push_runs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    target_id   INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    started_ms  INTEGER,
    finished_ms INTEGER,
    duration_ms INTEGER,
    -- `running`, `success`, `failed` ou `lost`.
    status      TEXT    NOT NULL,
    exit_code   INTEGER,
    -- Sortie du travail, tronquée côté serveur (10 Ko).
    output      TEXT    NOT NULL DEFAULT ''
);

CREATE INDEX push_runs_by_target ON push_runs (target_id, id DESC);

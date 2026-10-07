-- Réglages d'instance qui ne méritent pas leur propre table : clé / valeur texte.
-- Premier usage : `update_check` (vérification de nouvelle version), `on` / `off`.
CREATE TABLE app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

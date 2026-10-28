-- Règles de prévision de remplissage (`kind = 'forecast'`) et de dérive par
-- rapport à la saison (`kind = 'drift'`). Leurs réglages vivent dans `params`.
--
-- Même réécriture de la contrainte CHECK que `0131_alert_rules_log_pattern.sql` :
-- SQLite ne la modifie pas en place, et les lignes existantes la respectent.
PRAGMA writable_schema = ON;

UPDATE sqlite_schema
SET sql = replace(
    sql,
    'CHECK (kind IN (''threshold'', ''anomaly'', ''predict'', ''log_pattern''))',
    'CHECK (kind IN (''threshold'', ''anomaly'', ''predict'', ''log_pattern'', ''forecast'', ''drift''))'
)
WHERE type = 'table' AND name = 'alert_rules';

PRAGMA writable_schema = RESET;

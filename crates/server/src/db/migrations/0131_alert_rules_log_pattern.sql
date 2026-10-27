-- Règles « motif de journal » : une requête LogsQL, une fenêtre et un seuil de
-- nombre de lignes (`kind = 'log_pattern'`). La fenêtre et le découpage par
-- cible vivent dans `params`.
--
-- SQLite ne modifie pas une contrainte CHECK en place ; on réécrit son texte
-- dans le schéma, comme `0043_operator_role.sql` (élargir la liste des valeurs
-- ne change rien au format des pages, et les lignes existantes la respectent).
PRAGMA writable_schema = ON;

UPDATE sqlite_schema
SET sql = replace(
    sql,
    'CHECK (kind IN (''threshold'', ''anomaly'', ''predict''))',
    'CHECK (kind IN (''threshold'', ''anomaly'', ''predict'', ''log_pattern''))'
)
WHERE type = 'table' AND name = 'alert_rules';

PRAGMA writable_schema = RESET;

CREATE INDEX IF NOT EXISTS idx_alert_rules_kind ON alert_rules(kind);

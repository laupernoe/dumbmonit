-- Deux lectures du cycle d'alerting scannaient toute leur table faute d'index :
-- la purge des empreintes obsolètes (`last_eval_at`) et le chargement des
-- baselines saisonnières, dont le seau (`bucket`) est la seconde colonne de la
-- clé primaire `WITHOUT ROWID` et ne peut donc pas servir de point d'entrée.

CREATE INDEX idx_alert_state_last_eval ON alert_state(last_eval_at);

CREATE INDEX idx_anomaly_baselines_bucket ON anomaly_baselines(bucket, series_key);

-- Note de sécurité devenue facultative (réglage `security_score`, coupé par
-- défaut, voir `crate::security`). Une instance existante qui surveillait déjà
-- un équipement noté la voyait sur la fiche de cet équipement : elle la garde,
-- allumée. Une nouvelle instance n'a aucune cible à ce stade et reste coupée.
INSERT INTO app_settings (key, value)
SELECT 'security_score', 'on'
WHERE EXISTS (
    SELECT 1 FROM targets
    WHERE kind IN ('agent', 'proxmox', 'pbs', 'synology', 'opnsense', 'http', 'tls',
                   'activedirectory')
)
ON CONFLICT(key) DO NOTHING;

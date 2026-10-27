-- Mode multi-clients (MSP), suite : les objets ajoutés après la migration 0185
-- appartiennent eux aussi à un client ou, `client_id` NULL, à l'instance.
--
-- - tableaux de bord partagés (un tableau personnel reste à son propriétaire) ;
-- - plannings et politiques d'escalade d'astreinte ;
-- - procédures (runbooks) et actions de remédiation.
--
-- Tout l'existant reste à l'instance : rien ne change tant que l'administrateur
-- MSP ne range pas ces objets dans un client. Supprimer un client supprime les
-- siens, comme pour les canaux et les règles.

ALTER TABLE dashboards ADD COLUMN client_id INTEGER REFERENCES clients(id) ON DELETE CASCADE;
ALTER TABLE oncall_schedules
    ADD COLUMN client_id INTEGER REFERENCES clients(id) ON DELETE CASCADE;
ALTER TABLE oncall_policies
    ADD COLUMN client_id INTEGER REFERENCES clients(id) ON DELETE CASCADE;
ALTER TABLE runbooks ADD COLUMN client_id INTEGER REFERENCES clients(id) ON DELETE CASCADE;
ALTER TABLE remediation_actions
    ADD COLUMN client_id INTEGER REFERENCES clients(id) ON DELETE CASCADE;

-- Rapports SLA mensuels : un calendrier de rapports peut produire, au lieu du
-- résumé glissant, le rapport de disponibilité du mois civil précédent, avec le
-- PDF et/ou le CSV en pièces jointes.
--
-- `kind` : `digest` (le résumé historique) ou `sla` (mois civil, mensuel
-- seulement, validé par l'API). `page_id` : la page de statut dont le nom
-- habille le rapport. `objective` : objectif de disponibilité en pour cent
-- (99.9), pour indiquer pour chaque moniteur s'il est tenu.
ALTER TABLE report_schedules ADD COLUMN kind TEXT NOT NULL DEFAULT 'digest'
    CHECK (kind IN ('digest', 'sla'));
ALTER TABLE report_schedules ADD COLUMN attach_pdf INTEGER NOT NULL DEFAULT 1;
ALTER TABLE report_schedules ADD COLUMN attach_csv INTEGER NOT NULL DEFAULT 1;
ALTER TABLE report_schedules ADD COLUMN page_id INTEGER REFERENCES status_pages(id) ON DELETE SET NULL;
ALTER TABLE report_schedules ADD COLUMN objective REAL;

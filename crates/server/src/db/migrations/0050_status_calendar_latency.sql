-- Pages de statut : deux affichages publics facultatifs, coupés par défaut.
--
-- `show_calendar` : calendrier de 90 jours (une case par jour).
-- `show_latency`  : petit graphique de temps de réponse par service (24 h / 7 j).
ALTER TABLE status_pages ADD COLUMN show_calendar INTEGER NOT NULL DEFAULT 0;
ALTER TABLE status_pages ADD COLUMN show_latency INTEGER NOT NULL DEFAULT 0;

-- "Clear" pour une alerte déjà résolue dans l'historique : la ligne reste (c'est
-- la mémoire de l'instance), mais elle cesse d'apparaître dans les vues actives
-- une fois marquée. Une suppression définitive reste possible par ailleurs ;
-- ce drapeau ne fait que la masquer.
ALTER TABLE alert_history ADD COLUMN dismissed INTEGER NOT NULL DEFAULT 0 CHECK (dismissed IN (0, 1));

CREATE INDEX idx_alert_history_dismissed ON alert_history(to_phase, dismissed);

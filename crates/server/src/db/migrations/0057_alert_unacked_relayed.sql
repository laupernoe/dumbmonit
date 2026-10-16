-- Relais vers le canal d'escalade (alerte non acquittée), daté à part de la
-- dernière notification : une première annonce partie après le délai (fin de
-- maintenance) ou une escalade de sévérité au même cycle ne doit plus le faire
-- passer pour envoyé.
ALTER TABLE alert_state ADD COLUMN unacked_relayed_at TEXT;

-- Alertes déjà en cours à la mise à jour : le relais se lisait jusqu'ici dans
-- `last_notified_at`. On reprend cette valeur pour ne pas renvoyer, d'un coup,
-- un relais pour chaque alerte non acquittée déjà relayée.
UPDATE alert_state SET unacked_relayed_at = last_notified_at
 WHERE phase = 'firing' AND notify_count > 0;

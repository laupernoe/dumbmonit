-- Relais vers le canal d'escalade (alerte non acquittée), daté à part de la
-- dernière notification : une première annonce partie après le délai (fin de
-- maintenance) ou une escalade de sévérité au même cycle ne doit plus le faire
-- passer pour envoyé.
ALTER TABLE alert_state ADD COLUMN unacked_relayed_at TEXT;

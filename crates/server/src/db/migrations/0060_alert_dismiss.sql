-- « J'ai compris » : l'alerte en cours sort des vues actives et se tait.
--
-- Plus radical que l'acquittement : rien ne part plus pour cette occurrence,
-- ni rappel, ni escalade, ni résolution, et l'alerte disparaît de la liste,
-- de la vue d'ensemble, du mur et des compteurs. Comme l'acquittement « jusqu'à
-- la résolution », il est lié à l'occurrence : la condition retombée, il
-- s'efface, et une alerte qui revient plus tard est une alerte neuve
-- (voir `alerting/machine.rs`).
ALTER TABLE alert_state ADD COLUMN dismissed_at TEXT;   -- instant du geste, RFC 3339 UTC
ALTER TABLE alert_state ADD COLUMN dismissed_by TEXT;   -- compte (ou `token:…`) qui l'a écartée

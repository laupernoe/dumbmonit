-- Modules d'un agent devenus des équipements à part entière.
--
-- Un agent décrit sa machine (processeur, mémoire, disques, réseau, santé du
-- système) et, à côté, des services qu'il détecte seul : Docker, Plakar, ZFS,
-- libvirt, tunnels… Chacun de ces services devient une cible enfant de l'hôte
-- (`kind = 'agent_service'`, `parent_id` = l'hôte) : statut, alertes et règles
-- propres, et la suppression par dépendance quand l'hôte tombe.
--
-- Une ligne par (hôte, module) dès que le module a été vu une fois, ou qu'on
-- l'a coché ou décoché à la main.
CREATE TABLE agent_modules (
    host_target_id INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    module         TEXT    NOT NULL,
    -- NULL : automatique (l'appareil naît à la première détection).
    -- 1 : voulu, même sans détection. 0 : décoché, rien n'est créé ni gardé.
    enabled        INTEGER CHECK (enabled IN (0, 1)),
    -- Appareil du module, tant qu'il existe.
    target_id      INTEGER UNIQUE REFERENCES targets(id) ON DELETE SET NULL,
    -- Dernier lot qui contenait des mesures de ce module (ms depuis l'époque).
    -- Sa fraîcheur dit si le service répond encore.
    last_seen_ms   INTEGER,
    PRIMARY KEY (host_target_id, module)
) WITHOUT ROWID;

-- Supprimer un hôte emporte ses services ; supprimer l'appareil d'un service
-- revient à décocher son module — sans quoi le lot suivant le recréerait.
-- `BEFORE` : les lignes de l'hôte sont encore là pour retrouver ses enfants.
CREATE TRIGGER agent_modules_target_deleted BEFORE DELETE ON targets
BEGIN
    DELETE FROM targets WHERE id IN (
        SELECT target_id FROM agent_modules
        WHERE host_target_id = OLD.id AND target_id IS NOT NULL
    );
    UPDATE agent_modules SET enabled = 0, target_id = NULL WHERE target_id = OLD.id;
END;

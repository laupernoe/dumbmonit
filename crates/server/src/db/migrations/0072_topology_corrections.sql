-- Corrections de la carte de topologie, faites à la main, et règles apprises
-- de ces corrections.
--
-- Un nœud de la carte se désigne comme l'API le nomme : `t<id>` pour un
-- équipement, `g<hôte>:<nom>` pour un invité non surveillé, `n<hôte>:<nœud>`
-- pour un nœud d'un cluster Proxmox. `from_target` et `to_target` reprennent
-- l'équipement (ou l'hyperviseur) visé : supprimer l'équipement emporte ses
-- corrections, et un identifiant réutilisé plus tard ne les récupère pas.

CREATE TABLE topology_corrections (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    -- `forbid` : ce lien est faux, ne plus jamais le déduire ;
    -- `link`   : lien forcé, d'un type choisi ;
    -- `parent` : nouveau parent du nœud `to_node` (`from_node`) ;
    -- `site`   : site du nœud (`value`, vide = site principal) ;
    -- `group`  : regroupement du nœud sur la carte (`value`).
    kind            TEXT    NOT NULL CHECK (kind IN ('forbid', 'link', 'parent', 'site', 'group')),
    from_node       TEXT,
    to_node         TEXT    NOT NULL,
    from_target     INTEGER REFERENCES targets(id) ON DELETE CASCADE,
    to_target       INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    -- Type du lien interdit ou forcé (`firewall`, `lldp`, `manual`…).
    source          TEXT,
    value           TEXT,
    -- `parent` : le rattachement est aussi le parent d'alerte (`targets.parent_id`).
    use_for_alerts  INTEGER NOT NULL DEFAULT 0,
    -- Parent d'alerte d'avant la correction, rendu quand on la retire. Pas de
    -- clé étrangère : il peut avoir disparu entre-temps, on vérifie alors.
    previous_parent INTEGER,
    author          TEXT    NOT NULL DEFAULT '',
    created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX topology_corrections_to ON topology_corrections (to_node);

-- Règles généralisées. `status` = `active` (acceptée, appliquée aux nœuds
-- présents et futurs) ou `dismissed` (refusée : plus jamais proposée). `key`
-- résume la règle pour qu'une même suggestion ne revienne pas.
CREATE TABLE topology_rules (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    key           TEXT    NOT NULL UNIQUE,
    -- `forbid` : ne plus déduire les liens `source` vers ce périmètre ;
    -- `parent` : les nœuds de ce périmètre dépendent de `parent_node`.
    kind          TEXT    NOT NULL CHECK (kind IN ('forbid', 'parent')),
    status        TEXT    NOT NULL CHECK (status IN ('active', 'dismissed')),
    -- Périmètre : un sous-réseau (CIDR) ou une adresse, et un type d'équipement.
    subnet        TEXT,
    address       TEXT,
    target_kind   TEXT,
    source        TEXT,
    parent_node   TEXT,
    parent_target INTEGER REFERENCES targets(id) ON DELETE CASCADE,
    -- Nombre de corrections qui ont fait naître la règle.
    evidence      INTEGER NOT NULL DEFAULT 0,
    author        TEXT    NOT NULL DEFAULT '',
    created_at    TEXT    NOT NULL DEFAULT (datetime('now'))
);

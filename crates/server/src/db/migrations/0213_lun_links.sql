-- Lien manuel entre un LUN (ou un zvol) d'un NAS et ce qui l'utilise.
--
-- Un LUN iSCSI épais est opaque vu du NAS : son remplissage réel se lit chez
-- l'équipement qui l'a formaté (datastore vSphere, stockage Proxmox, disque
-- d'un agent). Quand le rapprochement automatique (nom + taille) n'est pas sûr,
-- l'opérateur dit « ce LUN est le disque X de l'équipement Y » : la famille de
-- capacité (`alerting/capacity.rs`) et l'entité (`/srv`, `datastore1`) telles
-- que `GET /targets/{id}/forecasts` les nomme.
--
-- `lun` est l'identifiant publié par le collecteur dans l'étiquette `lun`
-- (l'UUID de DSM, le nom du zvol). Supprimer le NAS ou l'équipement
-- consommateur emporte le lien.

CREATE TABLE lun_links (
    target_id           INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    lun                 TEXT    NOT NULL,
    consumer_target_id  INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    family              TEXT    NOT NULL,
    entity              TEXT    NOT NULL,
    author              TEXT    NOT NULL DEFAULT '',
    created_at          TEXT    NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (target_id, lun)
);

CREATE INDEX lun_links_consumer ON lun_links (consumer_target_id);

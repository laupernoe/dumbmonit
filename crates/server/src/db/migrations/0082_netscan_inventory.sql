-- Scans de découverte planifiés par sous-réseau, et inventaire de ce qu'ils
-- trouvent (`netinventory/`).
--
-- Un scan planifié balaie un sous-réseau à intervalle fixe, soit depuis le
-- serveur (`agent_id` NULL), soit depuis un agent relais. Le premier scan
-- terminé initialise l'inventaire sans alerter (`initialized`) ; ensuite, tout
-- appareil ou port absent de l'inventaire y entre « inconnu » (`known = 0`) et
-- reste signalé jusqu'à ce qu'un administrateur le marque connu.
CREATE TABLE netscan_schedules (
    id            INTEGER PRIMARY KEY,
    name          TEXT    NOT NULL DEFAULT '',
    cidr          TEXT    NOT NULL,
    agent_id      INTEGER REFERENCES targets(id) ON DELETE CASCADE,
    interval_secs INTEGER NOT NULL,
    depth         TEXT    NOT NULL DEFAULT 'standard',
    enabled       INTEGER NOT NULL DEFAULT 1,
    initialized   INTEGER NOT NULL DEFAULT 0,
    last_run_at   TEXT,
    last_status   TEXT,
    last_error    TEXT,
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now'))
);

-- Un même sous-réseau n'est planifié qu'une fois par exécutant.
CREATE UNIQUE INDEX idx_netscan_schedules_scope ON netscan_schedules(cidr, IFNULL(agent_id, 0));

-- Appareils connus. `ident` est `mac:<adresse>` quand la MAC est visible, sinon
-- `ip:<adresse>` : un appareil dont l'IP change garde sa MAC, donc son identité.
CREATE TABLE netscan_devices (
    id          INTEGER PRIMARY KEY,
    schedule_id INTEGER NOT NULL REFERENCES netscan_schedules(id) ON DELETE CASCADE,
    ident       TEXT    NOT NULL,
    mac         TEXT,
    ip          TEXT    NOT NULL,
    hostname    TEXT,
    vendor      TEXT,
    known       INTEGER NOT NULL DEFAULT 1,
    first_seen  TEXT    NOT NULL,
    last_seen   TEXT    NOT NULL,
    UNIQUE (schedule_id, ident)
);

CREATE INDEX idx_netscan_devices_unknown ON netscan_devices(known) WHERE known = 0;

-- Ports ouverts par appareil (services reconnus par le moteur de découverte).
CREATE TABLE netscan_ports (
    id         INTEGER PRIMARY KEY,
    device_id  INTEGER NOT NULL REFERENCES netscan_devices(id) ON DELETE CASCADE,
    port       INTEGER NOT NULL,
    protocol   TEXT    NOT NULL DEFAULT 'tcp',
    product    TEXT    NOT NULL DEFAULT '',
    known      INTEGER NOT NULL DEFAULT 1,
    first_seen TEXT    NOT NULL,
    last_seen  TEXT    NOT NULL,
    UNIQUE (device_id, port, protocol)
);

CREATE INDEX idx_netscan_ports_unknown ON netscan_ports(known) WHERE known = 0;

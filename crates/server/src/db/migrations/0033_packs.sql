-- Paquets d'intégration installés (`crates/pack`).
--
-- Le YAML est gardé tel qu'il a été fourni : c'est lui qui est relu et vérifié à
-- chaque démarrage, et exporté dans les sauvegardes. `sha256` est son empreinte,
-- comparée à la mise à jour pour dire « inchangé ». Un paquet désactivé reste
-- installé — ses règles et les équipements qui l'utilisent aussi — mais n'est
-- plus enregistré comme type de cible : ses équipements échouent en erreur de
-- configuration, sans alerte de panne.
CREATE TABLE packs (
    id           TEXT    PRIMARY KEY,
    version      TEXT    NOT NULL,
    yaml         TEXT    NOT NULL,
    sha256       TEXT    NOT NULL,
    enabled      INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    installed_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

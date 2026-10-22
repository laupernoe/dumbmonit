-- Recherche de failles (CVE) dans l'inventaire logiciel (`vulns/`).
--
-- Les sources externes (NVD, Debian Security Tracker, OSV.dev, CISA KEV)
-- sont gardées ici : hors ligne, le dernier état connu reste servi, et le
-- rafraîchissement quotidien ne redemande que ce qui a changé.

-- Dernier inventaire envoyé par chaque agent (`PushBatch.inventory`).
CREATE TABLE agent_inventory (
    target_id    INTEGER PRIMARY KEY REFERENCES targets(id) ON DELETE CASCADE,
    collected_at TEXT    NOT NULL,
    received_at  TEXT    NOT NULL,
    inventory    TEXT    NOT NULL
);

-- Ce qu'on affiche d'une CVE, quelle qu'en soit la source. `scored_at` :
-- dernière lecture de la fiche NVD (note CVSS).
CREATE TABLE cve_info (
    id        TEXT PRIMARY KEY,
    summary   TEXT NOT NULL DEFAULT '',
    cvss      REAL,
    published TEXT,
    scored_at TEXT
);

-- NVD : les CVE de chaque produit du catalogue, avec les plages de versions
-- (JSON) qui concernent ce produit.
CREATE TABLE cve_nvd_product (
    product TEXT NOT NULL,
    cve_id  TEXT NOT NULL,
    ranges  TEXT NOT NULL,
    PRIMARY KEY (product, cve_id)
);

-- Debian Security Tracker, réduit aux paquets et versions de l'inventaire.
CREATE TABLE cve_debian (
    package       TEXT    NOT NULL,
    release       TEXT    NOT NULL,
    cve_id        TEXT    NOT NULL,
    status        TEXT    NOT NULL,
    fixed_version TEXT,
    urgency       TEXT    NOT NULL DEFAULT '',
    nodsa         INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (package, release, cve_id)
);

-- OSV.dev : avis touchant une version donnée d'un paquet (liste d'identifiants
-- en JSON), et détail de chaque avis.
CREATE TABLE cve_osv_query (
    ecosystem  TEXT NOT NULL,
    package    TEXT NOT NULL,
    version    TEXT NOT NULL,
    vulns      TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    PRIMARY KEY (ecosystem, package, version)
);

CREATE TABLE cve_osv_vuln (
    id       TEXT PRIMARY KEY,
    modified TEXT NOT NULL DEFAULT '',
    data     TEXT NOT NULL
);

-- CISA KEV : failles exploitées.
CREATE TABLE cve_kev (
    cve_id     TEXT PRIMARY KEY,
    date_added TEXT    NOT NULL DEFAULT '',
    ransomware INTEGER NOT NULL DEFAULT 0,
    name       TEXT    NOT NULL DEFAULT ''
);

-- État de chaque source (`nvd`, `nvd:<produit>`, `debian`, `osv`, `kev`) :
-- `cursor` = borne du prochain différentiel.
CREATE TABLE cve_sources (
    name         TEXT PRIMARY KEY,
    last_attempt TEXT,
    last_success TEXT,
    last_error   TEXT,
    cursor       TEXT
);

-- Résultat du dernier calcul : composants inventoriés et verdicts.
CREATE TABLE cve_components (
    target_id INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    component TEXT    NOT NULL,
    version   TEXT    NOT NULL,
    origin    TEXT    NOT NULL,
    PRIMARY KEY (target_id, component)
);

CREATE TABLE cve_findings (
    target_id         INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    component         TEXT    NOT NULL,
    cve_id            TEXT    NOT NULL,
    installed_version TEXT    NOT NULL,
    fixed_version     TEXT,
    state             TEXT    NOT NULL,
    reason            TEXT    NOT NULL,
    source            TEXT    NOT NULL,
    first_seen        TEXT    NOT NULL,
    PRIMARY KEY (target_id, component, cve_id)
);

-- Risques acceptés : une CVE ignorée sur un équipement n'alerte plus et
-- n'est plus comptée.
CREATE TABLE cve_ignores (
    target_id  INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    cve_id     TEXT    NOT NULL,
    note       TEXT    NOT NULL DEFAULT '',
    created_by TEXT    NOT NULL DEFAULT '',
    created_at TEXT    NOT NULL,
    PRIMARY KEY (target_id, cve_id)
);

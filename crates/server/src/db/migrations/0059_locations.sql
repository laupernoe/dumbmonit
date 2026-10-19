-- Emplacements : là où se trouvent les équipements, pour la carte du monde.
--
-- Un emplacement a un nom, une adresse libre (affichée seulement) et des
-- coordonnées. Il peut tenir lieu d'un site de la carte de topologie (`site`,
-- comparé sans casse au site déduit de l'étiquette `site`, de l'agent ou du
-- relais) : tout ce qui est à ce site y est placé sans autre saisie. Un seul
-- emplacement peut être le site principal (`main_site`) : il reçoit ce qui n'a
-- aucun site distant.
--
-- `targets.location_id` est l'affectation explicite, qui passe avant le site.
CREATE TABLE locations (
    id         INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    address    TEXT    NOT NULL DEFAULT '',
    latitude   REAL    NOT NULL CHECK (latitude BETWEEN -90 AND 90),
    longitude  REAL    NOT NULL CHECK (longitude BETWEEN -180 AND 180),
    site       TEXT    NOT NULL DEFAULT '',
    main_site  INTEGER NOT NULL DEFAULT 0,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S', 'now'))
);

CREATE UNIQUE INDEX idx_locations_main ON locations(main_site) WHERE main_site = 1;

ALTER TABLE targets ADD COLUMN location_id INTEGER REFERENCES locations(id) ON DELETE SET NULL;

CREATE INDEX idx_targets_location ON targets(location_id);

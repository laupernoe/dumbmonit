-- Astreintes : plannings, couches de rotation et remplacements ponctuels.
--
-- La définition d'un planning (fuseau, couches, rotations, restrictions
-- horaires, participants) est un document JSON lu par `oncall::schedule` :
-- elle se modifie toujours d'un bloc depuis l'éditeur, et un schéma relationnel
-- de quatre tables n'apporterait que des jointures. Les participants y sont
-- des identifiants de comptes ; un compte supprimé est simplement ignoré au
-- calcul.
--
-- Les remplacements, eux, ont leur table : on les crée et on les retire un
-- par un, opérateurs compris, sans réécrire le planning entier.
CREATE TABLE oncall_schedules (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    description TEXT    NOT NULL DEFAULT '',
    definition  TEXT    NOT NULL DEFAULT '{}',
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- « X remplace Y du … au … ». `replaces_user_id` NULL : X prend la place de
-- quiconque serait d'astreinte. Bornes RFC 3339 UTC, fin exclue.
CREATE TABLE oncall_overrides (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    schedule_id      INTEGER NOT NULL REFERENCES oncall_schedules(id) ON DELETE CASCADE,
    user_id          INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    replaces_user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    starts_at        TEXT    NOT NULL,
    ends_at          TEXT    NOT NULL,
    note             TEXT    NOT NULL DEFAULT '',
    created_by       TEXT    NOT NULL DEFAULT '',
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    CHECK (ends_at > starts_at)
);
CREATE INDEX idx_oncall_overrides_schedule ON oncall_overrides(schedule_id, ends_at);

-- Dernière personne vue d'astreinte par planning : c'est ce qui permet
-- d'annoncer une prise ou une fin d'astreinte une seule fois.
CREATE TABLE oncall_shift_state (
    schedule_id INTEGER PRIMARY KEY REFERENCES oncall_schedules(id) ON DELETE CASCADE,
    user_id     INTEGER,
    since       TEXT    NOT NULL
);

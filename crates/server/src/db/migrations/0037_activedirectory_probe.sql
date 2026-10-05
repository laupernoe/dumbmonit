-- Dernière vue de chaque sonde Active Directory.
--
-- Les noms des membres des groupes privilégiés, les contrôleurs et leurs rôles,
-- et les constats de sécurité avec leurs exemples sont des libellés, pas des
-- nombres : VictoriaMetrics ne les garde pas. Le collecteur les livre à chaque
-- interrogation ; ils sont conservés ici pour que l'API les serve sans
-- réinterroger le contrôleur de domaine.
--
-- Une ligne par cible, réécrite à chaque interrogation : rien ne s'accumule, et
-- la cascade efface tout avec l'équipement.
CREATE TABLE activedirectory_probe_view (
    target_id INTEGER PRIMARY KEY REFERENCES targets(id) ON DELETE CASCADE,
    probed_at INTEGER NOT NULL,
    view      TEXT    NOT NULL
);

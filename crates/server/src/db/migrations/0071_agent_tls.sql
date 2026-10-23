-- Canal chiffré des agents : autorité interne, certificats clients, et canal
-- par lequel chaque machine a parlé en dernier.
--
-- Le serveur crée sa propre autorité au premier démarrage. Sa clé privée est
-- chiffrée par la clé de l'instance (`crypto.rs`) : une copie de la base seule
-- ne permet pas de fabriquer un certificat d'agent ni de se faire passer pour
-- le serveur.

-- Autorités du canal des agents. Une seule `active` à la fois ; `next` existe
-- pendant une rotation (les agents l'épinglent avant qu'elle ne signe quoi que
-- ce soit) ; `retired` reste reconnue pour les certificats clients qu'elle a
-- signés, jusqu'à leur échéance.
CREATE TABLE agent_ca (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    -- Nom commun du certificat : le sujet des certificats qu'elle signe doit le
    -- reprendre à l'identique.
    name         TEXT    NOT NULL,
    cert_der     BLOB    NOT NULL,
    -- Clé privée PKCS#8, chiffrée en AES-256-GCM.
    key_enc      BLOB    NOT NULL,
    -- SHA-256 hexadécimal du certificat : ce que les agents épinglent.
    fingerprint  TEXT    NOT NULL UNIQUE,
    state        TEXT    NOT NULL CHECK (state IN ('active', 'next', 'retired')),
    created_ms   INTEGER NOT NULL,
    retired_ms   INTEGER
);

-- Certificats clients délivrés. Le certificat n'est accepté que si sa ligne
-- existe : supprimer la machine (cascade) le révoque sur-le-champ, sans liste
-- de révocation à publier.
CREATE TABLE agent_certs (
    -- SHA-256 hexadécimal du certificat DER.
    fingerprint  TEXT    PRIMARY KEY,
    target_id    INTEGER NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    ca_id        INTEGER NOT NULL REFERENCES agent_ca(id) ON DELETE CASCADE,
    issued_ms    INTEGER NOT NULL,
    not_after_ms INTEGER NOT NULL,
    -- Remplacé par un renouvellement : encore accepté quelques minutes, le
    -- temps que les requêtes en vol de l'agent se terminent.
    replaced_ms  INTEGER
);

CREATE INDEX idx_agent_certs_target ON agent_certs(target_id);

-- Dernier lot reçu par le canal chiffré, avec le groupe d'échange de clés
-- négocié (`X25519MLKEM768` attendu), et dernier lot reçu en clair.
ALTER TABLE agent_hosts ADD COLUMN tls_seen_ms INTEGER;
ALTER TABLE agent_hosts ADD COLUMN tls_group TEXT;
ALTER TABLE agent_hosts ADD COLUMN legacy_seen_ms INTEGER;

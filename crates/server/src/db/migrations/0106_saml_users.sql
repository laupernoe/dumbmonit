-- Identité SAML des comptes.
--
-- `saml_subject` est le `NameID` persistant que l'IdP donne au compte,
-- `saml_issuer` l'`entityID` de cet IdP : le couple est l'identité stable, un
-- `NameID` n'étant unique que chez son émetteur. Un `NameID` transitoire n'est
-- jamais enregistré (il change à chaque connexion) : ces comptes se retrouvent
-- par leur adresse.
--
-- La configuration du fournisseur de service vit dans `settings` (clé `saml`).

ALTER TABLE users ADD COLUMN saml_subject TEXT;
ALTER TABLE users ADD COLUMN saml_issuer TEXT;

CREATE UNIQUE INDEX idx_users_saml_identity ON users(saml_issuer, saml_subject)
    WHERE saml_subject IS NOT NULL;

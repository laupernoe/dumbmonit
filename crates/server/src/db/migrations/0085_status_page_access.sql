-- Pages de statut privées : mode d'accès par page.
--
-- `public` : ouverte à tous (comportement historique). `password` : un mot de
-- passe propre à la page (empreinte Argon2id ci-dessous, jamais renvoyée par
-- l'API). `users` : réservée aux comptes DumbMonit connectés (mots de passe
-- locaux, LDAP ou SSO : toutes les sessions valent). Une page qui n'est pas
-- publique n'est jamais indexable ni listée dans le sitemap.
ALTER TABLE status_pages ADD COLUMN access_mode TEXT NOT NULL DEFAULT 'public'
    CHECK (access_mode IN ('public', 'password', 'users'));
ALTER TABLE status_pages ADD COLUMN access_password_hash TEXT;

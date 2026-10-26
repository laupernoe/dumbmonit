-- Pages de statut : réglages de référencement (bloc « SEO »).
--
-- `seo` : objet JSON {index, title, description, keywords, canonical, image,
-- lang, site_name}. `{}` : tout par défaut, donc page non indexable
-- (`noindex, nofollow`) et textes générés à partir du nom de la page.
ALTER TABLE status_pages ADD COLUMN seo TEXT NOT NULL DEFAULT '{}';

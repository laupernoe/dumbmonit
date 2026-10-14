-- Signature d'un changement de site (`webchange`) : l'empreinte des lignes
-- retirées et ajoutées. Une même modification sur plusieurs pages (un bandeau,
-- un paragraphe commun) donne la même signature, ce qui permet à l'interface
-- de la montrer une fois, « sur N pages ». NULL pour une page apparue ou
-- disparue, et pour les changements enregistrés avant cette colonne.
ALTER TABLE webchange_changes ADD COLUMN signature TEXT;

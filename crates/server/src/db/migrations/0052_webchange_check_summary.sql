-- Pages changées par la dernière vérification d'un site (`webchange`), en
-- quelques mots : « /tarifs, /contact and 3 more ». Portée par la mesure
-- `webchange_last_check_changes` en étiquette `changes`, elle dit dans la
-- notification ce qui a bougé, et fait de chaque vérification qui trouve
-- quelque chose une alerte nouvelle. NULL quand rien n'a changé.
ALTER TABLE webchange_state ADD COLUMN last_check_summary TEXT;

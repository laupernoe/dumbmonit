-- Mode produit et assistant de démarrage (`crate::instance`, clé `instance` de
-- `settings`). Une instance déjà configurée avant cette version garde
-- l'interface qu'elle connaît (`homelab`) et ne voit pas l'assistant de premier
-- lancement : il suffit qu'un compte ou un équipement existe. Une base neuve
-- n'a ni l'un ni l'autre et n'écrit rien ici ; l'assistant s'ouvrira après la
-- première connexion de l'administrateur.
INSERT INTO settings (key, value)
SELECT 'instance', CAST('{"product_mode":"homelab","setup_completed":true}' AS BLOB)
WHERE (EXISTS (SELECT 1 FROM users) OR EXISTS (SELECT 1 FROM targets))
  AND NOT EXISTS (SELECT 1 FROM settings WHERE key = 'instance');

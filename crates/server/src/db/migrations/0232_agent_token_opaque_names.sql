-- Les jetons d'enrôlement d'agents n'ont plus de nom lisible : un identifiant
-- opaque (`tok_` + 10 caractères base32 minuscules) tiré au hasard. On sait à quoi
-- sert un jeton par ses données (machines enrôlées), pas par son nom.
--
-- Idempotente : un jeton déjà au format `tok_…` n'est pas touché. `random()` est
-- réévaluée à chaque appel et chaque ligne, d'où dix tirages indépendants.
UPDATE agent_tokens
   SET name = 'tok_'
       || substr('abcdefghijklmnopqrstuvwxyz234567', (random() & 31) + 1, 1)
       || substr('abcdefghijklmnopqrstuvwxyz234567', (random() & 31) + 1, 1)
       || substr('abcdefghijklmnopqrstuvwxyz234567', (random() & 31) + 1, 1)
       || substr('abcdefghijklmnopqrstuvwxyz234567', (random() & 31) + 1, 1)
       || substr('abcdefghijklmnopqrstuvwxyz234567', (random() & 31) + 1, 1)
       || substr('abcdefghijklmnopqrstuvwxyz234567', (random() & 31) + 1, 1)
       || substr('abcdefghijklmnopqrstuvwxyz234567', (random() & 31) + 1, 1)
       || substr('abcdefghijklmnopqrstuvwxyz234567', (random() & 31) + 1, 1)
       || substr('abcdefghijklmnopqrstuvwxyz234567', (random() & 31) + 1, 1)
       || substr('abcdefghijklmnopqrstuvwxyz234567', (random() & 31) + 1, 1)
 WHERE name NOT GLOB 'tok_[a-z2-7][a-z2-7][a-z2-7][a-z2-7][a-z2-7][a-z2-7][a-z2-7][a-z2-7][a-z2-7][a-z2-7]';

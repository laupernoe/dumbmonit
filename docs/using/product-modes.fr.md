# Interface et assistant de démarrage

## La console

DumbMonit n'a qu'une seule interface : une console dense et très sombre, pour
les équipes qui la gardent ouverte toute la journée. Les administrateurs règlent
le nom de l'instance, la langue et le fuseau horaire dans **Réglages →
Général**.

- Un **rail d'icônes** à gauche liste toutes les destinations, pages de statut
  et rapports compris. Tout en bas : la documentation, GitHub, votre compte (le
  bouton rond avec vos initiales) et les Réglages. Sur téléphone, une barre
  d'onglets garde l'essentiel et **Plus**.
- L'**en-tête** dit où vous êtes (nom de l'instance, page, moniteur), avec un
  interrupteur **DIRECT** qui met en pause les rafraîchissements de la console,
  la période sur la vue d'ensemble, la palette de commandes et **Nouveau
  moniteur**.
- Le **panneau de contexte** à droite, façon inspecteur, montre ce qui est à
  traiter, le moniteur sélectionné dans un tableau (avec **Vérifier** et
  **Silence 1 h**), la prévision la plus proche et qui est prévenu. La touche
  `i` l'affiche ou le masque ; sur un écran plus étroit, il s'ouvre en tiroir.
- La **vue d'ensemble** montre cinq chiffres (moniteurs actifs, disponibilité
  sur 30 jours, temps de réponse p95, alertes ouvertes, temps moyen de
  résolution), la **rivière de santé** (une bande par moniteur, une case par
  tranche de la période, les pires en haut) et un **tableau des moniteurs** trié
  du pire au meilleur. Le tableau se filtre par état, type et texte, accepte des
  colonnes en plus (adresse, dossier) ou en moins, et se pilote au clavier :
  `j` / `k` pour se déplacer, `Entrée` pour sélectionner, `o` pour ouvrir.
- Le **mode mur** a son propre tableau : fond noir, la rivière de santé, les
  cinq chiffres, les alertes ouvertes et qui est d'astreinte, sans décor.
- Autres raccourcis : `g m` carte de topologie, `g x` sécurité, `g l` journaux.
  `?` affiche la liste complète.
- Les formulaires d'équipement ouvrent **Plus d'options** d'emblée et montrent
  plus de champs.

Un état est toujours écrit à côté d'une forme (rond actif, losange dégradé,
carré en panne), jamais indiqué par la seule couleur.

## Assistant de démarrage

L'assistant s'ouvre après la première connexion d'un administrateur sur une
nouvelle instance. Relancez-le quand vous voulez depuis **Réglages → Général →
Lancer l'assistant de démarrage**.

1. **Instance** : nom affiché dans l'en-tête, langue et fuseau horaire par
   défaut.
2. **Notifications** : ajoutez un canal (le même formulaire que dans les
   Réglages, donc tous les types de canaux) et envoyez un test.
3. **Premiers moniteurs** : balayez votre réseau sur place, ou ouvrez l'import
   et l'ajout à la main dans un nouvel onglet.
4. **Agent** : la commande d'installation pour une machine à surveiller.

Trois étapes facultatives suivent : **authentification
unique et équipes**, **astreinte** (politique de notification et rôles) et
**rétention**.

Chaque étape peut être passée, et **Passer la configuration** termine
l'assistant d'un coup. L'assistant est marqué terminé sur le serveur : il ne
revient pas dans un autre navigateur.

## API et sauvegarde

- `GET /api/instance` est public et ne répond que `product_mode` (toujours
  `enterprise`) et `setup_completed`.
- `GET /api/instance/settings` rend le nom, la langue et le fuseau ;
  `PUT /api/instance/settings` (administrateurs) les modifie. Un champ absent
  garde sa valeur.
- Ces réglages font partie de la sauvegarde de configuration (section
  `instance`). Restaurer une sauvegarde ne rouvre jamais l'assistant sur une
  instance où il a été fait.

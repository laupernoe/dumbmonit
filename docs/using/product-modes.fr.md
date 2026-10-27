# Interface et assistant de démarrage

DumbMonit a une seule interface : une console dense et sombre, faite pour
rester ouverte toute la journée. Les versions précédentes proposaient de choisir
entre une interface « Homelab » et une interface « Entreprise » ; il n'en reste
qu'une, et rien ne s'est perdu dans la fusion.

## La console

- Un **rail d'icônes** à gauche liste toutes les destinations, pages de statut et
  rapports compris. À son pied : la documentation, GitHub, votre compte (le
  bouton rond avec vos initiales) et les Réglages. Un téléphone a une barre
  d'onglets avec l'essentiel et **Plus**.
- L'**en-tête** indique où vous êtes (nom de l'instance, page, moniteur), un
  interrupteur **LIVE** qui met en pause les rafraîchissements de la console, la
  période sur la vue d'ensemble, la palette de commandes, **Nouveau moniteur**,
  le **mode mur** (l'icône d'écran) et le bouton du panneau de contexte.
- Le **panneau de contexte** à droite, comme un inspecteur, montre ce qui
  demande attention, le moniteur sélectionné dans un tableau (avec **Vérifier
  maintenant** et **Silence 1 h**), la prévision la plus proche et qui est
  prévenu. `i` l'affiche ou le masque ; sur un écran plus étroit, il s'ouvre en
  tiroir.
- La **vue d'ensemble** s'ouvre sur un fin bandeau de ciel nocturne dont
  l'unique ligne dit où en sont les choses (« pve-02 is down. 3 guests held
  back, alert sent. » ou « All 146 monitors up. », dans votre langue). En
  dessous : cinq chiffres (moniteurs en service, disponibilité sur 30 jours,
  temps de réponse p95, alertes ouvertes, temps moyen de résolution), la
  **rivière de santé** (une bande par moniteur, une case par tranche de la
  période, les pires en haut) et un **tableau des moniteurs** trié du pire au
  meilleur. Le tableau filtre par état, type et texte, permet d'ajouter des
  colonnes (adresse, dossier) et d'en retirer, et se pilote au clavier : `j` /
  `k` pour se déplacer, `Entrée` pour sélectionner, `o` pour ouvrir.
- Le **mode mur** est un tableau noir pour un écran dans la salle : la rivière
  de santé, les cinq chiffres, les alertes ouvertes et qui est d'astreinte.
- Raccourcis : `g o` vue d'ensemble, `g d` moniteurs, `g a` alertes, `g m`
  carte de topologie, `g x` sécurité, `g l` journaux. `?` affiche la liste
  complète.

Un état est toujours écrit à côté d'une forme (rond en service, losange
dégradé, carré hors service), jamais montré par la seule couleur. Les pages de
statut publiques gardent leurs thèmes clair et sombre.

## Assistant de démarrage

L'assistant s'ouvre après la première connexion de l'administrateur sur une
instance neuve. On peut le relancer à tout moment depuis **Réglages → Général →
Lancer l'assistant de démarrage**.

1. **Instance** : nom affiché dans l'en-tête, langue et fuseau horaire par
   défaut.
2. **Notifications** : ajouter un canal (le même formulaire que dans les
   Réglages, donc tous les types de canaux) et envoyer un test.
3. **Premiers moniteurs** : scanner le réseau sur place, ou ouvrir l'import et
   l'ajout à la main dans un nouvel onglet.
4. **Agent** : la commande d'installation pour une machine à surveiller.
5. Trois étapes facultatives : **authentification unique et équipes**,
   **astreinte** (politique de notification et rôles) et **rétention**.

Chaque étape peut être passée, et **Passer la configuration** termine
l'assistant d'un coup. L'assistant est marqué terminé sur le serveur : il ne
revient pas dans un autre navigateur.

## API et sauvegarde

- `GET /api/instance` est public et ne répond que `setup_completed`.
- `GET /api/instance/settings` renvoie le nom, la langue et le fuseau ;
  `PUT /api/instance/settings` (administrateurs) les modifie. Un champ absent
  garde sa valeur. L'ancien champ `product_mode` est ignoré si un client
  l'envoie encore.
- Ces réglages font partie de la sauvegarde de configuration (section
  `instance`). Une sauvegarde restaurée ne rouvre jamais l'assistant sur une
  instance où il était terminé, et une sauvegarde d'une version antérieure (avec
  `product_mode`) se restaure telle quelle.

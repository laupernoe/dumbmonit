# Interface et assistant de démarrage

DumbMonit a une seule interface : une console dense et sombre, faite pour
rester ouverte toute la journée. Les versions précédentes proposaient de choisir
entre une interface « Homelab » et une interface « Entreprise » ; il n'en reste
qu'une, et rien ne s'est perdu dans la fusion.

## La console

- Un **menu** à gauche liste quatre destinations : Vue d’ensemble,
  Surveillances, Alertes et Statut. Chacune a ses onglets (Surveillances :
  Liste, Carte, Journaux, Sécurité ; Alertes : Maintenant, Historique, Règles,
  Automatisation, Astreintes ; Statut : Pages, Rapports). À son pied : Réglages,
  mode mural, la documentation, GitHub et votre compte (le bouton rond avec vos
  initiales). Un téléphone a une barre d'onglets avec l'essentiel et **Plus**.
- L'**en-tête** indique où vous êtes (nom de l'instance, page, moniteur), un
  interrupteur **LIVE** qui met en pause les rafraîchissements de la console, la
  période sur la vue d'ensemble, la palette de commandes et **Nouvelle
  surveillance**.
- La **vue d'ensemble** s'ouvre sur un fin bandeau de ciel nocturne dont
  l'unique ligne dit où en sont les choses (« pve-02 is down. 3 guests held
  back, alert sent. » ou « All 146 monitors up. », dans votre langue). En
  dessous : quatre chiffres (surveillances en service, disponibilité sur 30
  jours, temps de réponse p95, alertes ouvertes), ce qui demande attention à
  côté de la **rivière de santé** (une bande par surveillance, les pires en
  haut) et deux graphiques. La liste complète est l'onglet **Liste** de
  Surveillances.
- Le **mode mural** est un tableau pour un écran dans la salle : le verdict et
  l'horloge, les problèmes, chaque surveillance, les chiffres du parc, la
  rivière de santé, qui est d'astreinte et, une fois configurée, la musique.
- Raccourcis : `g o` vue d'ensemble, `g d` surveillances, `g a` alertes, `g s`
  statut, `g m` carte, `g x` sécurité, `g l` journaux, `g n` nouvelle
  surveillance. `?` affiche la liste complète.

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

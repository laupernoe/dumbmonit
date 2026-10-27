# La page d'aperçu

La page d'accueil répond à « tout va-t-il bien ? » en une seconde environ, puis
vous laisse creuser à la demande.

## Le bandeau d'état {#the-bulletin}

Un bandeau en haut montre un ciel nocturne fixe : des étoiles qui scintillent, un
croissant de lune et un pigeon qui le traverse sans jamais se poser. Le ciel ne
change pas avec la météo ; la phrase à son pied dit où en sont les choses, en
mots. « Les 146 moniteurs sont actifs. » quand rien ne réclame votre attention,
sinon « pve-02 est en panne. » ou « pve-02 demande un coup d'œil. », suivi d'une
voix plus discrète sur ce que le système a fait (« 3 invités retenus, alerte
envoyée. »). Touchez le pigeon : il fait un tonneau. Avec
`prefers-reduced-motion`, le pigeon quitte le ciel et les étoiles restent
immobiles.

## Le guide de premier lancement {#the-first-run-guide}

Sur une instance neuve, la page sous le bandeau n'est pas vide : c'est un
guide en trois étapes — ajouter une surveillance, connecter un moyen d'être
averti, vérifier qu'un message arrive. Chaque étape est un clic, chacune
peut être écartée seule, et **Passer le guide** range les trois d'un coup.
Le choix est stocké sur l'instance, un second navigateur ne relance donc
pas le guide. Voir
[Ajouter votre première surveillance](../install/first-device.md#the-three-first-steps).

## Les cinq chiffres et la rivière de santé {#the-figures}

Sous le bandeau, cinq chiffres : **Moniteurs actifs** (sur le total, avec les
nombres en panne et en pause), **Disponibilité 30 j**, **Temps de réponse p95**,
**Alertes ouvertes** (critiques et avertissements) et **MTTR 30 j** (temps moyen
de résolution). La période choisie dans l'en-tête pilote le p95 et la rivière.

La **rivière de santé** est une bande par moniteur, une case par tranche de la
période, les pires moniteurs en haut. Un état est toujours écrit à côté d'une
forme (rond actif, losange dégradé, carré en panne), jamais indiqué par la seule
couleur.

## Le tableau des moniteurs {#the-monitor-table}

Le tableau liste chaque moniteur, du pire au meilleur. Il se filtre par état, type
et texte, accepte des colonnes en plus (adresse, dossier) ou en moins, et se
pilote au clavier : ++j++ / ++k++ pour se déplacer, ++enter++ pour sélectionner,
++o++ pour ouvrir.

Le moniteur sélectionné alimente le **panneau de contexte** à droite, façon
inspecteur : ce qui est à traiter (avec les actions sur les alertes, voir la
[page Alertes](alerts.md)), **Vérifier** et **Silence 1 h**, les prévisions les
plus proches et qui est d'astreinte. ++i++ l'affiche ou le masque ; sur un écran
plus étroit, il s'ouvre en tiroir. Pour la liste complète, voir la
[page Moniteurs](devices.md).

## Mode mur {#wall-mode}

`/wall` montre un tableau pour un écran de salle : le verdict, l'horloge, les cinq
chiffres, la rivière de santé, les alertes ouvertes et qui est d'astreinte.
Ouvrez-le avec l'icône **Mode mur** de la barre du haut ou la palette de
commandes ; ++esc++ ou **Quitter** retourne à la console. Voir
[Mode mur](wall.md).

## L'installer sur un téléphone {#install-it-on-a-phone}

L'interface fournit un manifeste web, un téléphone peut donc l'ajouter à
l'écran d'accueil : **Partager → Sur l'écran d'accueil** sur iOS,
**Installer l'appli** dans le menu de Chrome sur Android. Elle s'ouvre
alors sans l'habillage du navigateur, dans le thème réglé sur le
téléphone.

Il n'y a délibérément **aucun mode hors ligne et aucun service worker**.
Un écran de surveillance qui montre une page en cache est pire qu'un qui
dit qu'il ne peut pas atteindre le serveur : tout l'intérêt est que ce
que vous lisez est à jour. Sans worker, un serveur mis à jour est aussi
servi immédiatement, sans build périmé à vider.

## Palette de commandes {#command-palette}

++cmd+k++ sur macOS, ++ctrl+k++ ailleurs, ouvre la palette de commandes : un
champ, des résultats en trois groupes.

| Groupe | Entrées |
|---|---|
| Pages | Aperçu, Moniteurs, Alertes, Notifications, Pages de statut, Réglages, Mode mur, Documentation |
| Actions | Ajouter un moniteur, Scanner mon réseau, Planifier une maintenance, Ajouter un canal de notification, Nouvelle page de statut, Annoncer un incident |
| Moniteurs | Chaque moniteur, avec sa LED d'état ; ++enter++ l'ouvre |

## Navigation {#navigation}

Un **rail d'icônes** étroit à gauche liste chaque destination : Aperçu, Moniteurs,
Alertes (avec le nombre de ce qui vous réclame), Tableaux de bord, Explorer, la
carte de topologie, Sécurité, Journaux, Pages de statut, Rapports et Réglages. Au
pied : la documentation, GitHub et votre compte. L'en-tête dit où vous êtes, avec
l'interrupteur **DIRECT** qui met en pause les rafraîchissements de la console,
la période, la palette, **Mode mur** et **Nouveau moniteur**. Sur téléphone, le
rail devient une barre d'onglets avec l'essentiel et **Plus**.

- **Aperçu** : tout va-t-il bien, et qu'est-ce qui vous réclame.
- **Moniteurs** : chaque source que vous surveillez. Voir la
  [page Moniteurs](devices.md).
- **Alertes** : ce qui se déclenche, fenêtres de maintenance, historique et
  règles. Voir la [page Alertes](alerts.md).
- **Réglages** : votre compte, utilisateurs et authentification unique, jetons,
  canaux de notification, apparence, à propos. Voir [Réglages](settings.md).

L'interface est sombre, et rien n'y indique un état par la seule couleur : un
repère s'accompagne toujours d'un mot (Reporting, Unreachable, Advisory, Warning,
Suppressed by parent, Building up).

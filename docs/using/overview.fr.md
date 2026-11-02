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

## Les quatre chiffres {#the-figures}

Sous le bandeau, quatre chiffres : **Moniteurs actifs** (sur le total, avec les
nombres en panne et en pause), **Disponibilité 30 j**, **Réponse p95** et
**Alertes ouvertes** (critiques et avertissements). La période choisie dans
l'en-tête pilote le p95 et la rivière.

## Ce qui demande attention {#what-needs-attention}

La liste à côté de la rivière ne contient que ce qui est cassé maintenant, du
pire au moins grave, chaque ligne avec son action rapide : acquitter ou reporter
une alerte, ou mettre une surveillance en sourdine (la petite croix). Sans rien
de cassé, le pigeon dort et la carte le dit. À son pied, la **semaine à venir**
en une ligne : ce qui expire dans les sept jours. **Tout voir** ouvre les
[Alertes](alerts.md).

## La rivière de santé {#the-health-river}

Une bande par surveillance, une case par tranche de la période, les pires en
haut. La vue d'ensemble montre les huit premières ; **Voir toutes les
surveillances** ouvre l'onglet [Liste](devices.md) de Surveillances. Un état est
toujours écrit à côté d'une forme (rond actif, losange dégradé, carré en panne),
jamais indiqué par la seule couleur. Sur téléphone, la rivière s'ouvre d'un
toucher.

## Deux graphiques {#charts}

Plus bas : le **temps de réponse** de tout le parc sur la période (p95 et
médiane, avec les surveillances les plus lentes) et les **alertes des sept
derniers jours** (déclenchées et résolues).

## Mode mur {#wall-mode}

`/wall` montre un tableau pour un écran de salle : le verdict, l'horloge, les cinq
chiffres, la rivière de santé, les alertes ouvertes et qui est d'astreinte.
Ouvrez-le avec l'icône **Mode mur** en bas du menu de gauche ou la palette de
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
| Pages | Vue d’ensemble, Surveillances, Carte, Alertes, Automatisation, Notifications, Explorateur de métriques, Sécurité, Pages de statut, Rapports, Journaux, Réglages, Mode mural, Documentation |
| Actions | Ajouter un moniteur, Scanner mon réseau, Planifier une maintenance, Ajouter un canal de notification, Nouvelle page de statut, Annoncer un incident |
| Moniteurs | Chaque moniteur, avec sa LED d'état ; ++enter++ l'ouvre |

## Navigation {#navigation}

Un **menu** étroit à gauche liste quatre destinations : **Vue d’ensemble**,
**Surveillances**, **Alertes** (avec le nombre de ce qui vous réclame) et
**Statut**. Au pied : **Réglages**, **Mode mural**, la documentation, GitHub et
votre compte. Ce qui était une page à part est désormais un onglet de ces
entrées. L'en-tête dit où vous êtes, avec l'interrupteur **DIRECT** qui met en
pause les rafraîchissements de la console, la période, la palette et **Nouvelle
surveillance**. Sur téléphone, le menu devient une barre d'onglets avec
l'essentiel et **Plus**.

- **Vue d’ensemble** : tout va-t-il bien, et qu'est-ce qui vous réclame.
- **Surveillances** : chaque source que vous surveillez, en quatre onglets :
  **Liste**, **Carte**, **Journaux** et **Sécurité**. Voir
  [Surveillances](../monitors/index.md).
- **Alertes** : cinq onglets : **Maintenant**, **Historique**, **Règles**,
  **Automatisation** et **Astreintes**. Voir la [page Alertes](alerts.md).
- **Statut** : **Pages** (pages de statut publiques) et **Rapports**. Voir
  [Statut](../status/index.md).
- **Réglages** : votre compte, équipe et accès, supervision, notifications, le
  mur et le système. Voir [Réglages](../settings/index.md).

L'interface est sombre, et rien n'y indique un état par la seule couleur : un
repère s'accompagne toujours d'un mot (Reporting, Unreachable, Advisory, Warning,
Suppressed by parent, Building up).

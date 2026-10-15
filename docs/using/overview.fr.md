# La page d'aperçu

La page d'accueil répond à « est-ce que tout va bien ? » en une seconde
environ, puis laisse approfondir à la demande.

![L'aperçu](../assets/screenshots/overview-light.png){ loading=lazy }

## Le bulletin

La bande en haut est un bulletin météo pour votre réseau. Une phrase
résume le ciel : « Ciel dégagé. » quand rien n'a besoin de vous, sinon un
décompte de ce qui en a besoin (« 9 avis, 1 inaccessible. »). Dessous, des
plaques comptent les appareils qui rapportent et ceux inaccessibles, avec
l'heure de la dernière vérification, et trois indicateurs : **Rapportent**,
**Besoin de vous**, **Prévisions**.

La fenêtre météo à droite suit le réseau : dégagé, nuageux, couvert ou
tempête, selon le nombre d'alertes qui se déclenchent et leur gravité,
avec le pigeon qui traverse en vol. Sous `prefers-reduced-motion`, tout
reste immobile.

L'action principale unique de la page est **Ajouter un appareil**.

## Le guide de premier lancement

Sur une instance neuve, la page sous le bulletin n'est pas vide : c'est un
guide en trois étapes — ajouter un appareil, connecter un moyen d'être
averti, vérifier qu'un message arrive. Chaque étape est un clic, chacune
peut être écartée seule, et **Passer le guide** range les trois d'un coup.
Le choix est stocké sur l'instance, un second navigateur ne relance donc
pas le guide. Voir
[Ajouter votre premier appareil](../install/first-device.md#the-three-first-steps).

## Besoin de vous

La liste de ce qui a besoin de vous maintenant, du plus grave au moins
grave : appareils inaccessibles, puis alertes déclenchées par gravité.
Chaque ligne nomme la règle, l'appareil, la valeur actuelle, depuis
combien de temps ça dure, et si elle regroupe plusieurs séries (« ×4 »
pour quatre systèmes de fichiers sur le même appareil). Quatre actions :
**Acquitter** (acquitter cette alerte pour 1 h, 4 h, 24 h ou jusqu'à
résolution — les rappels s'arrêtent, la résolution est quand même
notifiée), **Reporter** (cacher juste cette alerte pour 1 h, 8 h, 1 jour
ou jusqu'à résolution), **Ignorer** (empêcher cette règle de jamais
alerter à nouveau sur cet appareil, de façon réversible), et **Ouvrir
l'appareil**.

Les alertes acquittées et reportées rejoignent leur propre groupe plus
discret sous la liste et quittent le compte *Besoin de vous*. Une alerte
ignorée disparaît complètement — elle n'est pas « connue », elle est
désactivée pour cet appareil. Voir
[Se débarrasser d'une alerte](alerts.md#getting-rid-of-an-alert).

## Résumé des appareils

Une ligne : combien d'appareils, combien rapportent, combien sont
inaccessibles, et un lien vers la page Appareils complète. L'aperçu ne
liste jamais les appareils : le rack vit sur [Appareils](devices.md).

## Prévisions et dernières 24 heures

**Prévisions** montre ce qui est prédit plutôt qu'observé : l'anomalie de
la ligne de base saisonnière, les extrapolations « plein dans quatre
jours », l'estimation de remplissage de PBS, et les règles dont le nom
annonce une prédiction. Tandis qu'une ligne de base apprend, ses entrées
le disent.

**Dernières 24 heures** liste les transitions de déclenchement de la
journée, avec un lien vers l'historique complet sur la page Alertes.

## Mode mur {#wall-mode}

`/wall` montre le bulletin seul, plein écran, pour un moniteur de salle :
la phrase, les compteurs et la liste *Besoin de vous*, rafraîchis toutes
les 20 secondes, avec l'écran maintenu allumé. ++esc++ ou **Quitter**
revient à l'aperçu. Ouvrez-le depuis la palette de commandes (« Mode
mur ») ou en tapant l'URL. Il peut aussi jouer de la musique lancée
depuis votre téléphone : voir [Mode mur et musique](wall.md).

![Mode mur](../assets/screenshots/wall-light.png){ loading=lazy }

## L'installer sur un téléphone

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

## Palette de commandes

++cmd+k++ sur macOS, ++ctrl+k++ ailleurs, ouvre la palette de commandes :
un champ de saisie, des résultats en trois groupes.

| Groupe | Entrées |
|---|---|
| Pages | Aperçu, Appareils, Alertes, Notifications, Pages de statut, Réglages, Mode mur, Documentation |
| Actions | Ajouter un appareil, Scanner mon réseau, Planifier une maintenance, Ajouter un canal de notification, Nouvelle page de statut, Annoncer un incident, Basculer le thème |
| Appareils | Chaque appareil, avec sa LED d'état ; ++enter++ l'ouvre |

## Navigation

La barre du haut porte Aperçu, Surveillances, Alertes (avec le compte de ce
qui a besoin de vous) et Réglages, le raccourci de la palette et le
basculement de thème. Sur un téléphone, elle devient une barre d'onglets en
bas avec ces quatre onglets et **Plus**, qui contient le mode mur et la
documentation.

- **Aperçu** — le bulletin : est-ce que tout va bien, et qu'est-ce qui a
  besoin de vous.
- **Surveillances** — le rack ; toutes les sources que vous surveillez. Voir
  [Page Appareils](devices.md).
- **Alertes** — ce qui se déclenche, les fenêtres de maintenance,
  l'historique et les règles. Voir [Page Alertes](alerts.md).
- **Réglages** — votre compte, les utilisateurs et l'authentification
  unique, les jetons d'agent et d'assistant, les canaux de notification et les
  rapports, les pages de statut publiques, l'apparence, à propos. Voir
  [Réglages](settings.md). Rien dans l'interface ne porte le statut par
la couleur seule : une plaque ou une LED s'accompagne toujours d'un mot
(Rapporte, Inaccessible, Avis, Avertissement, Supprimé par le parent, En
apprentissage).

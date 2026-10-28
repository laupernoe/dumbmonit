# Mode mur {#wall-mode-and-music}

`/wall` montre le tableau propre à la console, en plein écran, pour un moniteur
qui reste allumé dans une salle. Il se rafraîchit seul et garde l'écran éveillé.

De haut en bas :

- le **verdict** en quelques mots (« Tout va bien », « 3 problèmes ») avec la
  date et l'horloge, et l'heure de la dernière mise à jour ;
- les **cinq chiffres** de l'aperçu : moniteurs actifs, disponibilité sur 30
  jours, temps de réponse p95, alertes ouvertes et temps moyen de résolution ;
- la **rivière de santé**, la grande image du tableau : une bande par moniteur,
  les pires en haut ;
- les **alertes ouvertes**, les plus graves d'abord ;
- **qui est d'astreinte** maintenant, avec le planning et la fin de la garde.

Pas de musique, pas de widgets : fond noir (décor facultatif ci-dessous), la typographie de la
console, des états écrits à côté d'une forme et jamais indiqués par la seule
couleur.

## Ouvrir et quitter

Ouvrez-le avec l'icône **Mode mur** de la barre du haut, depuis la palette de
commandes (« Mode mur ») ou en tapant l'URL. ++esc++ ou **Quitter** retourne à la
console. Un compte limité à certaines équipes ou à certains clients n'a pas le
mur, puisqu'il montre toute la flotte.

## Décor {#backdrop}

Le mur partage l'apparence sombre de la console. Le bouton **Décor** (en haut à
droite, à côté de Quitter) peut dessiner l'une des villes de nuit des
[pages de statut](status-pages.md) derrière le verdict et l'horloge : Paris,
New York, Tokyo ou Dubaï, ou *Auto* pour la plus proche du fuseau horaire du
navigateur. Il est désactivé par défaut et mémorisé par navigateur ;
`?city=paris` (ou `newyork`, `tokyo`, `dubai`, `none`) dans un lien le choisit
pour cet écran seulement. Le ciel suit le tableau : des étoiles quand tout va
bien, des nuages sur un avis, un orage sur un avertissement. Sur un téléphone,
ou quand l'onglet est masqué, la ville reste immobile. Un `zermatt` enregistré
ou dans un lien arrive désormais sur Dubaï.

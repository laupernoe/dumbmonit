# Carte du monde

La vue **Monde** de la page Carte (`/map?view=world`) montre vos emplacements
sur une carte du monde : une pastille par emplacement, avec l'état des
moniteurs qu'il réunit. À côté de la carte, les mêmes emplacements sont listés,
les plus mal en point d'abord ; un clic sur une pastille ou une ligne affiche
ses moniteurs, chacun avec son état et un lien vers sa page.

Une pastille dit son état de trois façons, jamais par la seule couleur :

| Pastille | Sens |
|---|---|
| Rouge, croix | Au moins un moniteur est **en panne** ou injoignable. |
| Ambre, `!` | Un moniteur est mal configuré ou attend sa première réponse. |
| Turquoise, coche | Tous les moniteurs répondent. |
| Grise, tiret | Ses moniteurs sont en pause ou jamais sondés. |

Le chiffre sur la pastille est le nombre de moniteurs, ou *problèmes / total*
quand quelque chose ne va pas (`2/5`).

## Emplacements

Les emplacements se règlent dans **Réglages → Surveillance → Emplacements**
(administrateurs). Un emplacement a un nom, une adresse facultative (texte
libre, seulement affiché) et un endroit, choisi de trois façons :

- **Chercher un lieu** : tapez une ville ; le serveur interroge le géocodage
  d'Open-Meteo (le même service que la météo du mur) et vous choisissez une des
  réponses. Votre navigateur ne contacte jamais Open-Meteo.
- **Latitude / Longitude** : saisissez les coordonnées.
- **Cliquez sur la carte** à l'endroit de l'emplacement.

## Quels moniteurs un emplacement réunit

Un moniteur trouve son emplacement du plus explicite au plus déduit :

1. **Placé à la main** : le champ *Emplacement* de son formulaire (sous *Plus
   d'options*), ou **Placer à** dans le mode Sélection de la liste des
   moniteurs, pour plusieurs à la fois.
2. **Son site** : un emplacement peut *tenir lieu* d'un site de la
   [carte de topologie](topology.md) — l'étiquette `site` du moniteur, le site
   que déclare son agent ou son agent relais, ou le nom de l'agent relais qui
   l'interroge. Chaque moniteur de ce site y arrive sans être placé un par un.
3. **Le site principal** : un emplacement peut être marqué *Site principal*. Il
   réunit tous les moniteurs qui n'ont pas de site distant.

Rien n'est recopié : changez le relais d'un moniteur ou son étiquette `site`,
et il se déplace sur la carte au rafraîchissement suivant. *Automatique* dans le
formulaire (ou **Placer à → Automatique** en masse) retire un placement manuel.
Supprimer un emplacement garde ses moniteurs ; ils perdent seulement leur place.
Les moniteurs qu'aucun emplacement ne réunit sont comptés sous la liste.

## Fond de carte

Par défaut, la carte dessine les **contours intégrés** : les pays de
[Natural Earth](https://www.naturalearthdata.com/) (domaine public) au
1:110 millions, livrés avec DumbMonit. La carte ne contacte alors aucun serveur,
ce qui convient à une instance sans accès à Internet.

Deux autres fonds se choisissent sous **Fond de carte** :

- **Tuiles OpenStreetMap** : rues et villes. Chaque navigateur qui ouvre la
  carte charge les tuiles depuis `tile.openstreetmap.org`, envoie l'adresse de
  votre instance en référent (leur
  [politique d'usage des tuiles](https://operations.osmfoundation.org/policies/tiles/)
  le demande) et affiche *© OpenStreetMap contributors*. Convient à un usage
  ponctuel ; pour un mur ouvert toute la journée sur de nombreux écrans,
  utilisez votre propre serveur de tuiles.
- **Votre serveur de tuiles** : toute adresse `{z}/{x}/{y}` sur un seul hôte
  (pas de sous-domaines `{s}`), avec l'attribution qu'exige sa licence, affichée
  dans le coin de la carte.

L'hôte des tuiles choisi est ajouté à la politique de sécurité du contenu de
l'interface (`img-src`), et rien d'autre : les contours intégrés n'ont besoin
d'aucune exception.

## Clair, sombre, téléphone

La carte suit le thème ; les tuiles sont assombries en mode sombre. Sur un
téléphone, la liste passe sous la carte. Les pastilles s'atteignent au clavier
(Tab, puis Entrée).

## API

`GET /api/locations` renvoie les emplacements, leurs membres (avec la façon dont
chacun y est arrivé : `assigned`, `site` ou `main`), les sites connus et les
moniteurs sans emplacement. Voir l'[API HTTP](../reference/api.md) pour les
autres routes.

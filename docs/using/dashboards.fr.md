# Tableaux de bord et explorateur de métriques

Les **tableaux de bord** (`/dashboards`) sont des pages que vous composez vous-même
avec des widgets. L'**explorateur de métriques** (`/explore`) permet de parcourir ce
que DumbMonit conserve, d'essayer une requête et d'en placer le résultat sur un tableau.

## Tableaux de bord

Choisissez un tableau dans la liste, ou créez-en un avec **Nouveau tableau de bord**.
Un tableau est :

- **personnel** : seul son propriétaire le voit et le modifie, quel que soit son
  rôle (lecteurs et opérateurs compris) ;
- **partagé** : tout le monde le lit, les administrateurs le modifient. Seuls les
  administrateurs peuvent créer un tableau partagé ou activer / retirer le partage.

Chaque tableau enregistre sa **plage de temps** (15 minutes à 30 jours) et son
**rafraîchissement** (désactivé, 10 s, 30 s, 1 min, 5 min). En consultation, vous
pouvez changer les deux à la volée sans enregistrer ; les valeurs enregistrées
reviennent à la réouverture.

### Widgets

| Widget | Affiche |
|---|---|
| Graphique | Une requête MetricsQL sur la plage du tableau, dessinée avec le même graphique que les pages d'équipement. |
| Valeur | Un nombre issu d'une requête, avec un seuil d'**avertissement** et un seuil **critique** facultatifs. Indiquez si le problème est une valeur *trop élevée* ou *trop basse* ; le widget le dit aussi par écrit (« Au-dessus du seuil critique »), jamais par la couleur seule. |
| État d'un dossier | L'état de chaque moniteur d'un dossier, les pires d'abord, chacun avec un lien vers sa page. |
| Alertes actives | Les alertes en cours, les plus graves d'abord. |
| Note | Markdown simple : titres `#`, `**gras**`, `*italique*`, listes, `` `code` `` et liens. Le HTML n'est jamais interprété. |

### Modifier la disposition

Choisissez **Modifier le tableau**, ajoutez des widgets et placez-les sur la grille à
12 colonnes.

- **Souris et tactile** : faites glisser un widget par sa poignée, redimensionnez-le
  par la poignée du coin. Le contour en pointillés montre où il atterrira ; les autres
  lui font de la place.
- **Clavier** : placez le focus sur la poignée, puis les flèches déplacent le widget,
  Maj et les flèches le redimensionnent (ou les flèches sur la poignée du coin).
  Échap annule un glissement. Les changements sont annoncés aux lecteurs d'écran.
- **Téléphone** : la grille tient en une colonne. Les boutons *monter* et *descendre*
  réordonnent les widgets ; les tailles sont conservées pour les grands écrans.

Rien n'est enregistré avant **Enregistrer le tableau**.

## Explorateur de métriques

L'explorateur liste les métriques `dumbmonit_*` vivantes (écrites dans la dernière
heure) avec leur nombre de séries et, pour la métrique choisie, ses étiquettes et
quelques valeurs. Choisir une métrique remplit la requête ; choisir une valeur
d'étiquette l'ajoute comme filtre.

La zone de requête complète les noms de métriques à la frappe, les noms d'étiquettes
dans `{…}` et leurs valeurs après `étiquette="`. `Ctrl+Espace` liste les suggestions,
`Ctrl+Entrée` lance la requête. Le résultat est un graphique et un tableau de la
dernière valeur de chaque série. Les requêtes passent par le proxy de DumbMonit,
jamais directement par VictoriaMetrics.

**Ajouter à un tableau de bord** place la requête sur un tableau que vous pouvez
modifier, ou sur un nouveau tableau personnel, en graphique ou en valeur.

## Droits en bref

| Action | Lecteur | Opérateur | Administrateur |
|---|---|---|---|
| Lire les tableaux partagés et l'explorateur | oui | oui | oui |
| Créer, modifier, supprimer ses tableaux **personnels** | les siens | les siens | les siens |
| Créer, modifier, supprimer les tableaux **partagés** | non | non | oui |

Les jetons d'API ne voient que les tableaux partagés ; un jeton `write` peut les gérer.
Les tableaux font partie des sauvegardes : les partagés avec
l'instance, les personnels avec le nom de compte de leur propriétaire.

## API

`GET/POST /api/dashboards`, `GET/PUT/DELETE /api/dashboards/{id}` et
`GET /api/metrics/catalog` sont décrits dans la spécification OpenAPI.

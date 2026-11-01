# Explorateur de métriques

L'**explorateur de métriques** (`/explore`) permet de parcourir ce que DumbMonit
conserve et d'essayer une requête. On l'ouvre depuis la palette de commandes (`Ctrl+K`).

L'explorateur liste les métriques `dumbmonit_*` vivantes (écrites dans la dernière
heure) avec leur nombre de séries et, pour la métrique choisie, ses étiquettes et
quelques valeurs. Choisir une métrique remplit la requête ; choisir une valeur
d'étiquette l'ajoute comme filtre.

La zone de requête complète les noms de métriques à la frappe, les noms d'étiquettes
dans `{…}` et leurs valeurs après `étiquette="`. `Ctrl+Espace` liste les suggestions,
`Ctrl+Entrée` lance la requête. Le résultat est un graphique et un tableau de la
dernière valeur de chaque série. Les requêtes passent par le proxy de DumbMonit,
jamais directement par VictoriaMetrics.

## Droits en bref

Tout compte connecté peut utiliser l'explorateur ; pour un compte limité par des
équipes ou des clients, les séries sont filtrées aux équipements qu'il voit.

## API

`GET /api/metrics/catalog` est décrit dans la spécification OpenAPI.

!!! note
    Les tableaux de bord personnalisés ont été retirés. Leurs données restent dans
    la base mais ne sont plus lues, affichées ni exportées ; les anciennes
    sauvegardes qui en contiennent se restaurent sans eux.

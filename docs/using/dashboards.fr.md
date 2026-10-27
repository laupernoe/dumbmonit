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

### Partir d'un modèle

**Nouveau depuis un modèle** propose des tableaux prêts à l'emploi qui se remplissent
avec vos propres équipements. Choisissez-en un, indiquez ce qu'il couvre, vérifiez
l'aperçu (données en direct, rien n'est encore enregistré), nommez-le et créez-le.
Ensuite c'est un tableau ordinaire, que vous pouvez modifier.

| Modèle | Lit |
|---|---|
| Vue d'ensemble homelab | Tous les équipements : combien sont joignables ou en panne, alertes actives, état de chaque dossier, CPU et mémoire des hôtes avec agent. |
| Serveurs Linux et Windows | Hôtes avec agent : CPU, mémoire, disques par volume, charge, trafic réseau, services arrêtés. |
| Réseau | Switchs SNMP (trafic et erreurs par port), pare-feux FortiGate et OPNsense (CPU, passerelles, tunnels, états), latence Internet UniFi. |
| Proxmox et virtualisation | Nœuds, invités et stockages Proxmox VE, âge de la dernière sauvegarde de chaque invité. |
| Stockage et sauvegardes | Synology, TrueNAS, Proxmox Backup Server, Kopia et Duplicati : volumes et pools, datastores, temps écoulé depuis la dernière sauvegarde. |
| Sites web et TLS | Moniteurs HTTP, TLS et site web : disponibilité, temps de réponse, expiration des certificats. |
| Active Directory | Réponse des contrôleurs de domaine, réplication, comptes verrouillés ou obsolètes, constats de sécurité. |
| Kubernetes | Nœuds, pods, boucles de crash, réplicas manquants, événements d'avertissement. |
| Hôtes Docker | Conteneurs en marche et arrêtés, sur les hôtes dont l'agent remonte Docker. |
| SLA par dossier ou client | Disponibilité sur 7 et 30 jours, indisponibilité estimée, équipements les plus fragiles, état de chaque dossier. |

**Ce qu'il couvre.** Filtrez par type d'équipement (quand le modèle en lit plusieurs),
dossier (sous-dossiers compris), site (l'emplacement où l'équipement est placé sur la
carte du monde) et, en mode multi-clients, client. Seuls les équipements que votre
compte voit sont utilisés : un compte restreint obtient un modèle rempli de ses seuls
équipements.

**Un modèle sans aucun équipement pour l'alimenter est grisé**, avec la raison :
ajoutez d'abord un équipement du type qu'il lit. Un widget qui lit un type que vous
n'avez pas est omis du résultat.

La requête de chaque widget désigne les équipements par leur identifiant au moment de
la création. C'est un instantané : un équipement ajouté plus tard n'y figure pas tant
que vous ne recréez pas le tableau (ou n'ajoutez pas son identifiant aux requêtes).
Au-delà de 300 équipements, les requêtes portent les 300 premiers.

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

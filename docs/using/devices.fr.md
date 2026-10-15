# La page des surveillances

![Le rack : une façade par surveillance](../assets/screenshots/devices-light.png){ loading=lazy }

## La liste

La page **Surveillances** montre chaque surveillance dans l'une de deux dispositions,
mémorisée dans votre navigateur :

- **Compact** (par défaut) : une ligne par surveillance — voyant d'état, nom,
  type, adresse, dernière fois vu et état. Les enfants sont en retrait sous
  leur parent et s'atténuent quand le parent est inaccessible.
- **Tuiles** : une carte par surveillance, avec une sparkline d'une métrique
  représentative. Un enfant nomme la surveillance derrière laquelle il se trouve.

Les surveillances sont regroupées par dossier ; l'en-tête d'un dossier n'affiche
son état que si quelque chose à l'intérieur n'est pas actif. Les
administrateurs réordonnent les surveillances en les faisant glisser (disposition
Compact, liste ni filtrée ni triée) ou au clavier (Espace, flèches, Espace).

| État | Signification |
|---|---|
| Actif | La dernière sonde a réussi, dans les trois dernières périodes. |
| Inaccessible | La surveillance n'a pas répondu (délai dépassé, connexion refusée), ou aucune sonde depuis plus de trois périodes (90 s au moins). |
| Mal configuré | La dernière sonde a échoué de notre côté : mauvais identifiants, adresse ou option invalide, réponse inattendue. Affiché dans la couleur avis avec l'erreur ; ne déclenche aucune alerte. |
| En attente | Ajouté, pas encore sondé. |
| Désactivé | En pause : non vérifié, ne déclenche aucune alerte. |

Une seule barre d'outils regroupe tout :

- la zone de **recherche** (nom, adresse ou type) ;
- **Filtrer**, qui ouvre les filtres d'état (Tous, À traiter, Actifs,
  Désactivés), de type, de tri et d'étiquettes — le nombre sur le bouton
  compte ceux qui sont actifs, et **Effacer les filtres** les remet à zéro.
  Les filtres vivent dans l'adresse : une liste filtrée se met en favori ou se
  partage ;
- **Compact** / **Tuiles** ;
- **Sélectionner** (administrateurs), qui affiche une case à cocher par
  surveillance et une barre flottante en bas avec les actions groupées (activer,
  désactiver, déplacer dans un dossier, supprimer) ; **Terminé** ou Échap
  quitte le mode ;
- **⋯** (plus d'actions) : la **Carte** de topologie, **Créer un dossier**,
  **Importer des sites (CSV)** et **Importer depuis Uptime Kuma** ;
- **Ajouter une surveillance**, l’action principale.

## Une surveillance

![Page d'une surveillance : état, actions, alertes, le panneau spécifique au type, métriques](../assets/screenshots/device-light.png){ loading=lazy }

La page d'une surveillance montre sa plaque d'état, son adresse, son type, le
profil détecté, les étiquettes, la dernière sonde, et pour les services
l'état du certificat. Actions :

| Action | Effet |
|---|---|
| Sonder maintenant | Lance une sonde immédiatement et indique combien d'échantillons et de séries elle a produits. La première chose à essayer quand une surveillance n'affiche aucune donnée. |
| Modifier | Ouvre le même formulaire que pour l'ajout, prérempli. Laisser l'identifiant vide conserve celui enregistré. |
| Silence 1 h / Silence jusqu'à… | Une fenêtre de maintenance ponctuelle sur cette surveillance. |
| Supprimer | Demande « Supprimer définitivement ? » en place pendant cinq secondes, puis supprime la surveillance. Ses séries restent dans VictoriaMetrics jusqu'à expiration de la rétention. |

En dessous : **Alertes sur cette surveillance** — chaque alerte active propose
*Acquitter*, *Repousser* et *Ignorer*, et une règle ignorée sur cette surveillance
est listée séparément avec **Arrêter d'ignorer** — puis **Métriques** sur une
plage de temps (1 h, 6 h, 24 h, 7 jours), un graphique par nom de métrique avec
une ligne par série. Voir
[Se débarrasser d'une alerte](alerts.md#getting-rid-of-an-alert).

Les services (HTTP, TCP, DNS, ping, TLS) obtiennent des instruments à la
place : **Disponibilité** sur les dernières 24 heures ou 7 jours, **Temps de
réponse**, le nombre de **Vérifications**, et une barre d'**Historique** de
créneaux (le plus ancien à gauche) montrant quand le service était en panne.

## Politiques de conteneur

Sur une machine avec un agent et Docker, un bandeau **Docker** se trouve sous
l'en-tête : nombre de conteneurs, combien tournent, mises à jour disponibles,
et combien de conteneurs ont une politique. **Gérer les conteneurs** ouvre la
liste ; **Politiques…** ouvre un tableau avec deux interrupteurs par conteneur
et une ligne « Appliquer à tous » :

- **Redémarrer si arrêté** — le serveur redémarre le conteneur dans la minute
  qui suit son arrêt (au plus une fois toutes les dix minutes) ;
- **Mise à jour automatique dans les fenêtres de maintenance** — quand le
  registre a une image plus récente pour le même tag, le conteneur est recréé
  pendant une fenêtre de maintenance planifiée (page Alertes → Maintenance) et
  annulé s'il échoue son test de santé.

Chaque interrupteur s'enregistre seul. Détails et règles de sécurité :
[Conteneurs Docker](../devices/agent.md#docker-containers).

## Ajout et modification

Voir [Ajouter votre première surveillance](../install/first-device.md) pour le
formulaire et les [types de surveillance](../devices/index.md) pour ce que chaque
type demande. En modification, le formulaire ouvre **Plus d'options** de
lui-même si la surveillance en utilise déjà une.

## Suppression

Supprimer une surveillance le retire de la configuration, efface ses alertes sur le
champ — rien n'est notifié, l'historique garde une ligne
`surveillance supprimée ou désactivée` — et supprime ses séries temporelles du
VictoriaMetrics embarqué quelques secondes plus tard (au mieux : un échec est
seulement journalisé, et les séries vieillissent ensuite avec la rétention de
12 mois). Mettre une surveillance en pause efface ses alertes de la même façon ;
ses séries restent.

# La page des appareils

![Le rack : une façade par appareil](../assets/screenshots/devices-light.png){ loading=lazy }

## Le rack

Chaque appareil est une façade de rack 1U : une LED d'état, le nom, l'étiquette
du type, l'adresse, la dernière fois qu'il a été vu, et une sparkline d'une
métrique représentative. Les enfants s'empilent sous leur parent et
s'atténuent quand le parent est inaccessible.

| État | Signification |
|---|---|
| Actif | La dernière sonde a réussi, dans les trois dernières périodes. |
| Inaccessible | L'appareil n'a pas répondu (délai dépassé, connexion refusée), ou aucune sonde depuis plus de trois périodes (90 s au moins). |
| Mal configuré | La dernière sonde a échoué de notre côté : mauvais identifiants, adresse ou option invalide, réponse inattendue. Affiché dans la couleur avis avec l'erreur ; ne déclenche aucune alerte. |
| En attente | Ajouté, pas encore sondé. |
| Désactivé | En pause : non vérifié, ne déclenche aucune alerte. |

Filtres : une zone de recherche, un segment d'état (Tous, À traiter, Actifs,
Désactivés) et un filtre de type. **Ajouter un appareil** est l'action
principale.

Le bouton **Sélectionner** (opérateurs et administrateurs) affiche une case à
cocher par appareil et une barre flottante en bas avec les actions groupées ;
**Terminé** ou Échap quitte le mode.

## Un appareil

![Page d'un appareil : état, actions, alertes, le panneau spécifique au type, métriques](../assets/screenshots/device-light.png){ loading=lazy }

La page d'un appareil montre sa plaque d'état, son adresse, son type, le
profil détecté, les étiquettes, la dernière sonde, et pour les services
l'état du certificat. Actions :

| Action | Effet |
|---|---|
| Sonder maintenant | Lance une sonde immédiatement et indique combien d'échantillons et de séries elle a produits. La première chose à essayer quand un appareil n'affiche aucune donnée. |
| Modifier | Ouvre le même formulaire que pour l'ajout, prérempli. Laisser l'identifiant vide conserve celui enregistré. |
| Silence 1 h / Silence jusqu'à… | Une fenêtre de maintenance ponctuelle sur cet appareil. |
| Supprimer | Demande « Supprimer définitivement ? » en place pendant cinq secondes, puis supprime l'appareil. Ses séries restent dans VictoriaMetrics jusqu'à expiration de la rétention. |

En dessous : **Alertes sur cet appareil** — chaque alerte active propose
*Acquitter*, *Repousser* et *Ignorer*, et une règle ignorée sur cet appareil
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

Voir [Ajouter votre premier appareil](../install/first-device.md) pour le
formulaire et les [types d'appareil](../devices/index.md) pour ce que chaque
type demande. En modification, le formulaire ouvre **Plus d'options** de
lui-même si l'appareil en utilise déjà une.

## Suppression

Supprimer un appareil le retire de la configuration, efface ses alertes sur le
champ — rien n'est notifié, l'historique garde une ligne
`appareil supprimé ou désactivé` — et supprime ses séries temporelles du
VictoriaMetrics embarqué quelques secondes plus tard (au mieux : un échec est
seulement journalisé, et les séries vieillissent ensuite avec la rétention de
12 mois). Mettre un appareil en pause efface ses alertes de la même façon ;
ses séries restent.

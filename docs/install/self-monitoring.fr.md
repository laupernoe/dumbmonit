# Surveiller DumbMonit lui-même

Qui surveille le surveillant ? DumbMonit répond en deux temps :

- un **battement de cœur sortant**, pour ce qu'il ne peut pas voir de
  l'intérieur : l'hôte tombé, le conteneur arrêté, le moteur bloqué ;
- des **règles intégrées** sur sa propre santé, pour ce qu'il voit : le volume
  de données qui se remplit, la base de métriques qui redémarre en boucle, les
  interrogations qui se figent, les écritures qui échouent, la base qui grossit
  anormalement.

Les deux se trouvent dans **Réglages → Système → DumbMonit lui-même**, qui
montre aussi l'état du moteur à l'instant.

## Le battement de cœur sortant

DumbMonit appelle une URL à vous à intervalle régulier (60 secondes par
défaut), **seulement tant que le moteur va bien** :

- le planificateur tourne (il a terminé un tour dans les deux dernières
  minutes) ;
- la base SQLite répond ;
- VictoriaMetrics répond.

Si l'un d'eux flanche, rien ne part. Un service de type « dead man's switch »
attend ces appels et vous alerte quand ils cessent, que la cause soit l'hôte
entier, le conteneur ou un seul composant.

### Mise en place

1. Créez un contrôle sur le service de votre choix et copiez son URL de ping :
   - **Healthchecks.io** (ou un Healthchecks auto-hébergé) : un contrôle de
     période égale à l'intervalle, avec un délai de grâce un peu plus long
     (`https://hc-ping.com/<uuid>`) ;
   - **Uptime Kuma** : un moniteur de type *Push*
     (`https://kuma.example.com/api/push/<jeton>?status=up&msg=OK&ping=`) ;
   - **Better Stack** : un heartbeat (`https://uptime.betterstack.com/api/v1/heartbeat/<jeton>`) ;
   - tout service qui accepte un simple `GET` et alerte quand il n'arrive plus
     (Cronitor, Dead Man's Snitch…).
2. Dans **Réglages → Système → DumbMonit lui-même**, collez l'URL, choisissez
   l'intervalle (30 à 3600 secondes), activez **Envoyer un battement** et
   enregistrez.
3. Appuyez sur **Tester** : un appel part tout de suite et le résultat
   s'affiche. Le service doit marquer le contrôle comme actif.

L'appel est un `GET` avec un délai maximal de 10 secondes. Il suit la même
garde que les canaux de notification : il ne part jamais vers l'interface web
de DumbMonit ni vers son VictoriaMetrics.

!!! note "ntfy"
    ntfy distribue chaque message reçu : y envoyer le battement vous notifie
    chaque minute, et non quand les appels cessent. Utilisez un service de
    « dead man's switch », et gardez ntfy comme canal pour les alertes
    elles-mêmes.

### Signal d'échec

Par défaut, un moteur malade cesse simplement d'appeler, et le service alerte
une fois son délai de grâce écoulé. Activez **Signaler les pannes** pour
appeler plutôt l'URL d'échec, que le service signale aussitôt :

| Service | URL d'échec |
|---|---|
| Healthchecks.io, Better Stack | `/fail` ajouté au chemin : `https://hc-ping.com/<uuid>/fail` |
| Uptime Kuma | `status=up` remplacé par `status=down` dans la requête |

Laissez-le coupé pour tout autre service : une URL inconnue terminée par
`/fail` répondrait très probablement 404, ce qui revient à ne rien envoyer.

### L'URL est un secret

Une URL de ping contient le plus souvent son propre jeton : qui la connaît
peut faire paraître votre contrôle en bonne santé. DumbMonit la range chiffrée
avec le secret d'instance, ne la renvoie jamais par l'API (seul son hôte est
montré) et la tient hors des journaux et des messages d'erreur. Pour la
changer, saisissez la nouvelle ; pour la garder, laissez le champ vide.
**Retirer l'URL** l'oublie et coupe le battement.

Le réglage ne fait pas partie du paquet de configuration chiffré : refaites-le
après une restauration sur une nouvelle instance.

## Règles intégrées

Chaque minute, DumbMonit écrit quelques mesures sur lui-même dans
VictoriaMetrics, et les règles intégrées du groupe *DumbMonit lui-même* les
lisent. Elles se comportent comme toutes les autres : mêmes canaux, mêmes
silences, mêmes seuils réglables dans **Alertes → Règles**.

| Règle | Se déclenche quand | Par défaut |
|---|---|---|
| Volume de données de DumbMonit presque plein | Moins de 10 % d'espace libre sur le volume qui porte `/data` | 15 min, avis |
| Base de métriques de DumbMonit qui redémarre en boucle | Le VictoriaMetrics embarqué a été relancé 3 fois ou plus en 30 minutes | 1 min, critique |
| Planificateur de DumbMonit bloqué | Aucun tour de planification depuis plus de deux minutes | 5 min, critique |
| Interrogations de DumbMonit en retard | Des équipements restent en retard après chaque tour pendant 15 minutes | 15 min, avis |
| Écritures de métriques de DumbMonit en échec | Des lots refusés par VictoriaMetrics, ou des mesures perdues, dans les 15 dernières minutes | 5 min, avis |
| Base de DumbMonit qui grossit vite | `dumbmonit.db` a plus que doublé en un jour et dépasse 100 Mo | 1 h, avis |
| La sauvegarde de DumbMonit n'a pas tourné | Aucune sauvegarde locale réussie depuis deux jours ([détails](backup.md#scheduled-local-backups)) | 1 h, avis |

Ces règles ont besoin de VictoriaMetrics pour être évaluées. Quand
VictoriaMetrics lui-même est arrêté pour de bon, ou l'hôte entier, aucune
règle ne peut se déclencher : c'est le rôle du battement de cœur.

### Mesures

| Métrique | Signification |
|---|---|
| `dumbmonit_instance_data_free_bytes` | Espace libre sur le volume qui porte `/data`, pour un utilisateur non privilégié. |
| `dumbmonit_instance_data_size_bytes` | Taille de ce volume. |
| `dumbmonit_instance_database_bytes` | Taille de `dumbmonit.db` et de son journal d'écriture. |
| `dumbmonit_instance_scheduler_lag_seconds` | Secondes depuis la fin du dernier tour du planificateur (moins de 2 en temps normal). |
| `dumbmonit_instance_scheduler_backlog` | Équipements encore dus après le dernier tour. |
| `dumbmonit_instance_sample_writes_failed_total` | Lots refusés par VictoriaMetrics ou restés sans réponse, depuis le démarrage du serveur. |
| `dumbmonit_instance_samples_dropped_total` | Mesures abandonnées avant d'atteindre le tampon d'écriture, depuis le démarrage du serveur. |
| `dumbmonit_instance_vm_restarts_total` | Relances du VictoriaMetrics embarqué depuis le démarrage du serveur (embarqué seulement). |

Elles ne portent pas d'étiquette `target` : les alertes qu'elles lèvent ne
sont rattachées à aucun équipement. Pour votre propre Prometheus, `GET /metrics`
expose le même genre de chiffres, voir
[Collecter DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## API

| Méthode | Route | |
|---|---|---|
| `GET` | `/api/self-monitoring` | État du moteur, volume de données, taille de la base, réglage et dernier battement. |
| `PUT` | `/api/self-monitoring/heartbeat` | `{ "enabled", "interval_seconds", "send_fail", "url" }`. `url` omise la garde, `""` l'efface. |
| `POST` | `/api/self-monitoring/heartbeat/test` | Un appel tout de suite, vers `{ "url" }` ou l'URL rangée. |

Les deux écritures sont réservées aux administrateurs (ou à un jeton de portée
`write`).

# Proxmox Datacenter Manager

La console qui fédère plusieurs clusters Proxmox VE et instances Proxmox
Backup Server : quelles instances elle atteint, tout le parc d'un coup
d'œil, les tâches qui ont échoué n'importe où, et la propre santé de la
console.

!!! warning "Testé contre une instance de test seulement"

    Cette intégration a été construite et vérifiée contre un Proxmox
    Datacenter Manager 1.1 installé pour l'occasion, pas contre un en
    production. Chaque appel est une lecture, et les formes viennent d'un
    vrai datacenter manager, mais personne ne l'a encore fait tourner
    pendant des semaines sur un datacenter manager en fonctionnement.
    Dites-nous ce qui casse.

## Une surveillance pour le parc, ou une surveillance par cluster ?

Une surveillance Datacenter Manager ne remplace pas les surveillances
[Proxmox VE](proxmox.md) et [Proxmox Backup Server](pbs.md). Il répond à
une question différente.

| | Surveillance Datacenter Manager | Une surveillance par cluster et par serveur de sauvegarde |
|---|---|---|
| Surveillances à créer | un | un par cluster, un par serveur de sauvegarde |
| Identifiants à gérer | un jeton, sur la console | un jeton par instance |
| Charge sur chaque cluster | aucune de notre part : nous lisons le propre cache de la console | une sonde par intervalle, par cluster |
| Ce que vous obtenez | accessibilité, version, compteurs d'invités et de nœuds, totaux de CPU, mémoire et stockage, tâches en échec | tout cela plus les invités un par un, snapshots, jobs de sauvegarde, HA, Ceph, réplication, disques et SMART, pools ZFS, le calendrier de sauvegarde sur 30 jours |
| Ce que vous ne pouvez pas obtenir | séries par invité, détail par datastore, disques, snapshots, journaux de tâche | un total unique à l'échelle du parc |
| Panne principale | la console ne peut pas atteindre une instance | le cluster ne nous répond pas |

**Notre recommandation.** Ajoutez la surveillance Datacenter Manager quand vous
faites déjà tourner une console et voulez une page qui dit si le parc est
complet — c'est la façon la moins coûteuse de remarquer qu'un site
distant a disparu, et ça ne coûte rien aux clusters. Ajoutez une surveillance
dédiée pour chaque cluster et serveur de sauvegarde que vous exploitez
réellement : c'est là que se trouve la profondeur, et c'est ce qui vous
dit *quelle* VM s'est arrêtée ou *quelle* sauvegarde a échoué. Les deux se
recoupent sans entrer en conflit ; une sauvegarde en échec lève alors une
alerte depuis la surveillance PBS (avec le journal de tâche) et une depuis la
console (nommant l'instance). Réduisez au silence la règle de tâche de la
console si vous trouvez ça redondant, ou limitez-la aux instances pour
lesquelles vous n'avez pas de surveillance dédiée.

Si vous ne faites pas tourner de console du tout, n'en installez pas une
juste pour la surveillance : les surveillances dédiées voient strictement plus.

## La page de la surveillance

Au-delà des graphiques génériques, une surveillance Datacenter Manager montre
trois panneaux, lus depuis ce que la sonde a stocké — ouvrir la page
n'interroge ni la console ni les clusters qu'elle gère :

* **Instances fédérées** — une bande de totaux de parc (instances, nœuds
  en ligne, invités en cours et arrêtés, cœurs, mémoire, stockage), puis
  une ligne par instance fédérée. Les instances que la console ne peut
  pas atteindre se trient en premier, puis celles avec des tâches en
  échec, puis celles en retard de version. Chaque ligne donne le produit
  (Proxmox VE ou Backup Server), la version, les compteurs de nœud et
  d'invité, l'usage mémoire et stockage, l'état d'abonnement, l'adresse
  configurée, quand la console a collecté pour la dernière fois depuis
  elle — et, quand ça a échoué, le message que la console elle-même a
  reçu. Ce message est la seule chose qui explique la panne, il est donc
  montré mot pour mot.
* **Échecs** — chaque tâche qui a échoué sur tout le parc dans les 14
  derniers jours, la plus récente d'abord, avec l'instance sur laquelle
  elle a tourné, le type de tâche, quand elle a commencé, combien de
  temps elle a pris et le message d'erreur. Le journal de tâche lui-même
  reste sur l'instance : ouvrez-le là, ou ajoutez ce cluster comme son
  propre surveillance, où *Montrer le journal* le récupère.
* **Hôte de la console** — la machine qui fait tourner la console : CPU,
  mémoire, système de fichiers racine, uptime et noyau, ses certificats
  avec les jours restants avant expiration, les paquets en attente d'une
  mise à jour, et le statut d'abonnement des nœuds gérés. Tout le panneau
  est optionnel : un jeton sans `Sys.Audit` n'en lit rien et le panneau le
  dit, plutôt que de montrer des zéros.

L'historique des tâches est gardé dans SQLite (`pdm_task_history`, 19
jours, 5000 tâches au plus par surveillance) pour que la liste d'échecs
survive aux redémarrages même si chaque sonde ne regarde que la fenêtre de
tâches.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_pdm_`.

| Famille | Métriques | Labels |
|---|---|---|
| Parc | `remotes_total`, `remotes_failed`, `nodes_online`, `nodes_offline`, `cpu_used_cores`, `cpu_total_cores`, `memory_used_bytes`, `memory_total_bytes`, `memory_used_percent`, `storage_used_bytes`, `storage_total_bytes`, `storage_used_percent`, `datastores_total` | |
| Invités du parc | `guests_running`, `guests_stopped` | `guest_type` (`qemu`, `lxc`) |
| Instances | `remote_reachable` (1 la console a eu une réponse, 0 non), `remote_version_info` (série de présence, label `version`), `remote_version_behind`, `remote_nodes_online`, `remote_nodes_offline`, `remote_guests_running`, `remote_guests_stopped`, `remote_cpu_used_cores`, `remote_cpu_total_cores`, `remote_cpu_used_percent`, `remote_memory_used_bytes`, `remote_memory_total_bytes`, `remote_memory_used_percent`, `remote_storage_used_bytes`, `remote_storage_total_bytes`, `remote_storage_used_percent`, `remote_datastores`, `remote_subscription_active`, `remote_updates_pending`, `remote_last_collection_age_seconds`, `remote_tasks_failed` | `remote`, `type` (`pve`, `pbs`) |
| Tâches dans la fenêtre | `tasks_total`, `tasks_failed`, `tasks_running` | |
| Hôte de la console | `node_cpu_percent`, `node_cpu_count`, `node_iowait_percent`, `node_load1`, `node_memory_used_bytes`, `node_memory_total_bytes`, `node_memory_used_percent`, `node_swap_used_bytes`, `node_swap_total_bytes`, `node_rootfs_used_bytes`, `node_rootfs_total_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_kernel_info`, `node_updates_pending`, `version_info` | |
| Certificats de la console | `node_certificate_expiry_days` (négatif une fois expiré) | `file` (`proxy.pem`) |
| Abonnement | `subscription_active` (label `status`), `subscription_active_nodes`, `subscription_total_nodes` | |
| Collection | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Une tâche qui se termine en `WARNINGS: n` compte comme réussie. Seul
l'échec de `GET /version` condamne toute la sonde ; tout autre appel en
échec est compté dans `scrape_errors`, et une instance que la console ne
peut pas atteindre est une mesure (`remote_reachable = 0`), pas une
erreur de sonde.

`remote_version_behind` compare les instances fédérées **entre elles**,
produit par produit : il est à 1 quand un autre cluster Proxmox VE — ou un
autre serveur de sauvegarde — dans le même parc tourne une version plus
récente. Ce n'est pas une comparaison contre ce que Proxmox publie ; une
sonde n'a pas à contacter l'internet. La série n'existe que pour une
instance dont la version a été lue, une instance inaccessible ne passe
donc jamais pour à jour.

Les totaux de stockage comptent chaque nom de stockage une fois par
instance : un stockage partagé par cinq nœuds d'un cluster est compté une
fois, pas cinq.

Règles intégrées qui s'appliquent : Surveillance inaccessible, instance
fédérée inaccessible, instance fédérée en retard, tâche en échec sur une
instance fédérée, disque de console de datacenter presque plein,
certificat de console de datacenter arrivant à expiration, mises à jour
de console de datacenter en attente. Les notifications nomment l'instance
concernée.

## Ce qu'il faut préparer dans Proxmox Datacenter Manager

Les étapes ci-dessous sont celles que montre l'avis à côté du formulaire.
Le principe : un utilisateur réservé à la surveillance, un jeton avec un
rôle en lecture seule — jamais le compte avec lequel vous vous connectez.

1. Dans la console, ouvrez Configuration → Access Control → Users et cliquez sur Add. Nommez le compte comme suit et laissez le mot de passe vide : le jeton est ce qui se connecte.

    ```
    dumbmonit@pdm
    ```

2. Toujours sous Access Control, ouvrez API Tokens → Add, choisissez cet utilisateur et nommez le jeton « monitor ». La console montre le secret une fois : copiez-le maintenant.

    ```
    dumbmonit@pdm!monitor
    ```

3. Donnez à la fois à l'utilisateur et au jeton le rôle `Auditor` sur `/` (le haut de l'arbre, donc tout le parc). `Auditor` porte `System.Audit`, `Resource.Audit` et `Access.Audit`, et ne peut rien changer. Un jeton n'a jamais plus de droits que son utilisateur, la permission est donc accordée deux fois : Permissions → Add → User Permission, puis à nouveau avec API Token Permission.
4. Copiez l'id du jeton dans le champ Token ID de DumbMonit et le secret dans son champ Secret.
5. Dans DumbMonit, saisissez l'adresse de la console, par exemple « dc.lan » ou « dc.lan:8443 ».

!!! warning
    Ne réutilisez pas le compte avec lequel vous vous connectez : un jeton qui fuit contrôlerait alors chaque cluster que la console fédère à la fois. Le rôle `Auditor` ne peut que lire. La console utilise aussi un certificat autosigné par défaut : si la connexion est refusée pour cette raison, cochez « Accepter un certificat non vérifiable » dans les options. Deux éléments ont besoin de plus qu'`Auditor` et sont simplement ignorés sans ça : le statut de collecte de métriques par instance, et le résumé de mise à jour des instances fédérées.

Vous préférez la ligne de commande ?
`proxmox-datacenter-manager-admin` gère les remotes, pas les
utilisateurs ; créez le compte, le jeton et les deux permissions dans
l'interface web.

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton d'API (recommandé) | **Token ID** : utilisateur, domaine et nom de jeton comme la console les montre, `dumbmonit@pdm!monitor`. **Secret** : l'UUID montré une fois à la création du jeton. DumbMonit les envoie comme l'en-tête `PDMAPIToken=user@pdm!name:secret` que la console attend. |

Il n'y a pas d'option nom d'utilisateur / mot de passe, et c'est
délibéré : depuis sa première release, la console ne renvoie son ticket de
session que dans un cookie `HttpOnly` — le corps JSON porte un
`ticket-info` inutilisable. Une connexion n'apporterait rien par rapport
à un jeton tout en ouvrant une session connectée à chaque sonde. Les
jetons sont ce que Proxmox recommande pour l'automatisation de toute
façon.

Droits, en lecture seule. Le plus simple et suffisant : le rôle intégré
`Auditor` sur `/`, accordé à l'utilisateur *et* au jeton. Par point de
terminaison :

| Point de terminaison | A besoin de |
|---|---|
| `GET /version` | tout utilisateur authentifié |
| `GET /remotes/remote` | renvoie les instances que l'appelant peut auditer |
| `GET /remotes/remote/{id}/version` | `Resource.Audit` sur `/resource/{id}` |
| `GET /resources/status`, `GET /resources/list` | `Resource.Audit` sur `/resource` |
| `GET /resources/subscription` | `Resource.Audit` sur `/resource` |
| `GET /remotes/tasks/list` | `Resource.Audit` sur `/resource/{remote}`, par instance |
| `GET /nodes/{node}/status` | `System.Audit` sur `/system/status` |
| `GET /nodes/{node}/certificates/info`, `GET /nodes/{node}/subscription` | tout utilisateur authentifié |
| `GET /nodes/{node}/apt/update` | `System.Audit` ; un 403 est toléré : aucune série `node_updates_pending`, aucune erreur de sonde |
| `GET /remotes/metric-collection/status` | plus qu'`Auditor` ; un 403 est toléré : aucune série `remote_last_collection_age_seconds`, aucune erreur de sonde |
| `GET /remotes/updates/summary` | `Resource.Modify` — un privilège d'écriture. Désactivé par défaut (`remote_updates`) pour cette raison |

Notez que la console masque les jetons d'accès qu'elle détient pour ses
instances (`GET /remotes/remote` renvoie un `token` vide), et DumbMonit
ne lit jamais ce champ de toute façon : aucun secret d'instance ne passe
jamais par le collecteur, la vue stockée ou l'API.

Adresse : `10.0.0.40`, `dc.lan`, `dc.lan:8443`, `[fd00::2]` ou une URL
complète. Le schéma `https` et le port 8443 sont ajoutés s'ils manquent.

## Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `port` | Port de l'API | `8443` | Utilisé si l'adresse ne donne pas de port. |
| `insecure_tls` | Accepter un certificat non vérifiable | `false` | Proxmox Datacenter Manager livre un certificat autosigné par défaut : activez ceci si la connexion est refusée pour cette raison. |
| `request_timeout_seconds` | Délai par requête (secondes) | `20` | Temps accordé à chaque appel d'API, de 1 à 120. La console relaie les appels vers chaque instance fédérée, un site lent retient donc toute la réponse. |
| `node` | Nom du nœud console | `localhost` | Nom du nœud qui fait tourner la console. Un Datacenter Manager a un seul nœud et l'appelle « localhost ». |
| `task_lookback_hours` | Fenêtre de tâches (heures) | `24` | Les tâches plus anciennes ne sont pas comptées parmi les échecs, de 1 à 8760. |
| `max_age_seconds` | Âge de cache accepté (secondes) | `60` | À quel point le propre inventaire de la console peut être périmé avant qu'elle n'interroge à nouveau les instances fédérées, de 0 à 3600. Zéro demande des données fraîches à chaque sonde, ce qui remet chaque cluster sur la ligne. |
| `remotes` | Instances surveillées | *(vide)* | Noms des instances fédérées à surveiller, séparés par des virgules. Vide : chaque instance. |
| `max_remotes` | Limite de version d'instance | `100` | Nombre maximum d'instances interrogées pour leur version à chaque sonde, de 1 à 1000. |
| `versions` | Lire la version de chaque instance | `true` | Un appel par instance fédérée, pour rapporter sa version et signaler celles en retard sur leurs pairs. Une instance qui ne répond pas compte comme inaccessible. |
| `tasks` | Surveiller les tâches à travers les instances | `true` | Lit la liste de tâches de chaque instance fédérée pour rapporter les échecs. |
| `node_status` | Surveiller l'hôte de la console | `true` | Lit le CPU, la mémoire, le système de fichiers racine, l'uptime, les certificats et l'abonnement de la machine qui fait tourner la console. |
| `updates` | Compter les mises à jour en attente | `true` | Liste les paquets en attente d'une mise à jour sur la console elle-même. |
| `remote_updates` | Compter les mises à jour sur chaque instance | `false` | Lit le résumé de mise à jour de la console pour les instances fédérées. Désactivé par défaut : la console protège ce chemin avec `Resource.Modify`, un privilège d'écriture qu'un compte de surveillance n'a aucune raison de détenir. |

`max_age_seconds` est celle qui vaut la peine d'être comprise. La console
garde son propre inventaire, rafraîchi par sa propre boucle de collecte.
Demander des données fraîches à chaque sonde fait que la console appelle à
nouveau chaque cluster, en plus de son rythme normal — exactement la
charge qu'une surveillance console unique est censée éviter. Soixante secondes
reste plus frais que l'intervalle de sonde par défaut.

## Erreurs courantes

| Symptôme | Cause probable |
|---|---|
| Erreur de certificat | Certificat autosigné : installez-en un de confiance (ACME est intégré à la console) ou activez `insecure_tls`. |
| Erreur d'authentification | Le secret collé dans Token ID (ou l'inverse), ou la permission a été accordée à l'utilisateur mais pas au jeton — un jeton n'a jamais plus de droits que son utilisateur, et les deux listes de contrôle sont des entrées séparées. |
| Chaque instance se montre inaccessible, mais la propre page de la console va bien | Le jeton a `Auditor` sur l'utilisateur mais pas sur le jeton, ou seulement sur `/system` : `Resource.Audit` sur `/resource` est ce qui laisse la console répondre pour ses instances. |
| Une instance est inaccessible et le message mentionne une empreinte | La console vérifie le certificat de chaque instance qu'elle gère ; corrigez-le dans la console (Remotes → l'instance), pas ici. |
| Aucune série `remote_last_collection_age_seconds` | `GET /remotes/metric-collection/status` a besoin de plus qu'`Auditor` et est ignoré silencieusement. Rien d'autre n'est affecté. |
| Aucune série `remote_updates_pending` | `remote_updates` est désactivé par défaut parce que ce chemin a besoin de `Resource.Modify`. Activez-le seulement si vous êtes prêt à accorder un privilège d'écriture. |
| Aucun panneau hôte de console | Le jeton manque `System.Audit` sur `/system/status`, ou `node_status` est désactivé. |
| « Federated instance behind » ne se déclenche jamais | Elle compare les instances du même produit entre elles. Une console qui fédère un seul cluster Proxmox VE n'a rien à lui comparer. |
| Les totaux de parc sont tous des tirets cadratins | La console a répondu mais n'a rien compté — généralement parce que chaque instance est inaccessible. Les lignes d'instance disent pourquoi. |

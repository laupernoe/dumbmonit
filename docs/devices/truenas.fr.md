# TrueNAS

Serveur de stockage ZFS (TrueNAS SCALE, maintenant TrueNAS Community
Edition) : la santé de chaque pool et le disque qui a échoué, scrubs et
resilvers, usage de pool et de dataset par rapport à leurs quotas,
snapshots et réplication, température des disques et autotests SMART, les
alertes que TrueNAS lève lui-même, services et la machine.

!!! warning "Testé contre une instance de test seulement"

    Cette intégration a été construite et vérifiée contre TrueNAS
    25.04.2.6, installé depuis l'ISO officiel dans une machine virtuelle
    pour l'occasion — un pool RAIDZ1, des datasets, des tâches de snapshot
    et de réplication, puis le même pool avec un disque mis hors ligne —
    pas contre un NAS qui garde les photos d'un foyer depuis des années.
    Une machine virtuelle n'a pas de températures de disque ni de
    résultats SMART, ceux-ci suivent donc le code source de TrueNAS plutôt
    qu'une réponse observée. Chaque appel est une lecture. Dites-nous ce
    qui casse.

## La panne visée

Un vdev mirror ou RAIDZ qui perd un disque continue de servir les données.
Le partage reste monté, les sauvegardes continuent de tourner, l'interface
web de chaque autre outil reste verte — et le pool n'a maintenant plus de
redondance. Le second disque emporte le pool. `truenas_pool_healthy` tombe
à 0 sur le premier, la règle **pool NAS dégradé** se déclenche, et la page
de l'appareil nomme le disque.

## La page de l'appareil

Au-delà des graphiques génériques, un appareil TrueNAS montre trois
panneaux, lus depuis ce que la sonde a stocké — ouvrir la page n'interroge
jamais le NAS lui-même :

* **Pools et disques** — chaque pool avec son état en un mot, la propre
  phrase de ZFS quand quelque chose ne va pas, son remplissage, sa
  disposition (mirror, RAIDZ, cache, log), ses compteurs d'erreurs de
  lecture, écriture et checksum, et surtout le nom de tout disque qui
  n'est pas `ONLINE`. Un scrub ou resilver en cours montre sa progression.
  Puis chaque disque : modèle, numéro de série, type, taille, pool,
  température et le résultat de son dernier autotest SMART.
* **Datasets et protection** — chaque dataset avec son usage par rapport à
  son quota, son nombre de snapshots et quand une tâche de snapshot l'a
  couvert pour la dernière fois ; le dernier scrub de chaque pool et s'il
  est en retard ; chaque tâche de réplication et de snapshot périodique
  avec son état, sa dernière exécution, le dernier snapshot qu'elle a
  traité et, quand elle a échoué, la propre phrase d'erreur de TrueNAS.
* **Santé du NAS** — les alertes que TrueNAS a levées lui-même, les plus
  graves d'abord, puis les services arrêtés, puis la machine : version,
  uptime, charge, mémoire, CPU.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_truenas_`.

| Famille | Métriques | Labels |
|---|---|---|
| Identité | `version_info` (la version, sans le préfixe `TrueNAS-SCALE-` que les anciennes releases mettaient devant) | `version` |
| Machine | `uptime_seconds`, `load1/5/15`, `memory_total_bytes`, `cpu_count` | |
| Pools | `pool_healthy` (le propre verdict de ZFS : 0 sur `DEGRADED` comme sur `FAULTED`), `pool_warning` (resilvering, fonctionnalités non activées), `pool_status_info`, `pool_size/allocated/free_bytes`, `pool_used_percent`, `pool_fragmentation_percent` | `pool`, et `status` pour la série d'info |
| Périphériques de pool | `pool_device_errors` (cumulé depuis le dernier `zpool clear`), `pool_devices_unhealthy` (les deux absents quand le pool ne peut pas être ouvert) | `pool`, `kind` (`read`, `write`, `checksum`) |
| Scrubs | `pool_scan_running`, `pool_scan_percent` (seulement pendant l'exécution), `pool_last_scrub_age_seconds`, `pool_last_scrub_errors`, `pool_scrub_threshold_days` | `pool`, `function` (`SCRUB`, `RESILVER`) |
| Datasets | `dataset_used_bytes`, `dataset_available_bytes`, `dataset_quota_bytes` et `dataset_quota_used_percent` (absent sans quota), `dataset_snapshots`, `dataset_snapshot_age_seconds` (âge du dernier snapshot pris par une tâche périodique couvrant le dataset ; absent quand aucune ne le fait), `dataset_locked` (datasets chiffrés seulement) | `dataset`, `pool` |
| Snapshots | `snapshots` (total) | |
| Disques | `disk_temperature_celsius`, `disk_size_bytes`, `disk_smart_failed` (absent pour un disque jamais testé) | `disk`, `serial` |
| Alertes | `alerts` — le nombre d'alertes actives par niveau, zéro inclus | `level` (`INFO` … `EMERGENCY`) |
| Tâches | `replication_error`, `replication_last_run_age_seconds`, `snapshot_task_error`, `snapshot_task_last_run_age_seconds` (tâches activées seulement) | `task` |
| Services | `service_running` (services réglés pour démarrer avec le NAS seulement) | `service` |
| Collection | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Quelques choses que fait TrueNAS qui valent d'être connues :

* **Le dernier scrub ne vit que dans l'enregistrement de scan du pool.**
  Il n'y a pas d'historique de scrub dans l'API, et un resilver écrase
  l'enregistrement de scan : après un resilver, la date du dernier scrub
  est simplement inconnue jusqu'à ce que le prochain se termine, et
  `pool_last_scrub_age_seconds` est absent plutôt qu'incorrect.
* **Un pool qui ne peut pas être ouvert** — exporté, ou avec trop de
  disques manquants — apparaît quand même, sans disposition ni tailles.
  Son `pool_healthy` est 0 ; les séries de périphérique et de taille sont
  absentes plutôt que zéro.
* **Un quota non réglé n'est pas un quota de zéro.** TrueNAS répond
  `parsed: null` pour « pas de quota » et DumbMonit ne publie aucune série
  de quota pour ce dataset. `quota` a priorité sur `refquota` quand les
  deux sont réglés.
* **Les compteurs de snapshot viennent du propre cache de ZFS**
  (`snapshots_count`), et le total du compteur rapide de snapshots de
  TrueNAS — aucun snapshot n'est jamais listé. Le total est plus grand que
  la somme des compteurs par dataset : il inclut les snapshots des datasets
  internes de TrueNAS, que la liste de datasets cache.
* **L'âge du snapshot le plus récent d'un dataset** est la dernière
  exécution de la tâche de snapshot périodique qui le couvre,
  directement ou via un parent récursif. TrueNAS ne peut pas donner la
  date du snapshot le plus récent sans les lister tous, donc un dataset
  qu'aucune tâche ne couvre n'a pas d'âge — ce qui est en soi à savoir.
* **Les températures sont lues depuis le cache de TrueNAS**
  (`only_cached`), rafraîchi par TrueNAS environ toutes les cinq minutes.
  Un disque endormi n'est jamais réveillé, et un disque sans lecture n'a
  aucune série.
* **Rien n'est jamais démarré.** Aucun scrub, aucun test SMART, aucune
  vérification de mise à jour, aucune réplication. La sonde lit.
* **Les chemins qui ont changé entre versions sont optionnels.**
  `zfs/snapshot` est devenu `pool/snapshot` en 25.10 et les deux sont
  essayés ; `smart/test/results` a disparu en 25.10 et est alors ignoré en
  silence.

Règles intégrées qui s'appliquent : Appareil inaccessible, pool NAS
dégradé, erreurs de disque NAS, pool NAS presque plein, scrub NAS a trouvé
des erreurs, scrub NAS en retard, disque NAS a échoué son test SMART,
disque NAS trop chaud, dataset NAS proche de son quota, réplication NAS
échouée, tâche de snapshot NAS échouée, snapshots NAS périmés, alerte
TrueNAS levée, service NAS arrêté. Les notifications nomment le pool, le
disque, le dataset, la tâche ou le service concerné.

## Ce qu'il faut préparer dans TrueNAS

Les étapes ci-dessous sont celles que montre l'avis à côté du formulaire.
Une chose à savoir d'abord : via l'API REST que DumbMonit utilise,
**TrueNAS n'a pas de clé en lecture seule**. Ses rôles en lecture seule
n'ont été branchés que dans sa nouvelle API WebSocket ; une clé dont
l'utilisateur est un Read-Only Administrator est refusée, avec un `403`
vide, à chaque appel REST. Cela a été vérifié sur TrueNAS 25.04.2.6 : une
clé Read-Only Administrator a obtenu `403` sur les 55 chemins REST
essayés, et la même clé fonctionnait via l'API WebSocket. La clé doit
appartenir à un utilisateur avec le privilège Local Administrator — elle
obtient donc un utilisateur et un groupe qui lui sont propres, et rien
d'autre.

1. Dans l'interface web : Credentials → Groups → Add. Nommez le groupe
   comme suit et donnez-lui le privilège Local Administrator.

    ```
    dumbmonit
    ```

2. Pourquoi Local Administrator et pas Read-Only Administrator : TrueNAS
   n'a branché ses rôles en lecture seule que dans son API WebSocket. Via
   l'API REST que DumbMonit utilise, une clé en lecture seule est refusée
   à chaque appel. DumbMonit lui-même ne fait toujours que lire : il ne
   démarre jamais un scrub, un test SMART, une mise à jour ou une
   réplication.

3. Credentials → Users → Add. Nommez le compte comme suit, faites de
   dumbmonit son groupe principal, et laissez l'accès shell, sudo et SSH
   désactivés : aucun n'est nécessaire.

    ```
    dumbmonit
    ```

4. Créez la clé : Credentials → Users → API Keys → Add, choisissez
   l'utilisateur dumbmonit et donnez un nom à la clé. TrueNAS ne montre la
   clé qu'une fois : copiez-la. Sur TrueNAS 24.10, le chemin est le menu
   utilisateur en haut à droite → API Keys → Add, et une clé n'appartient
   à aucun utilisateur.

5. Dans DumbMonit, saisissez l'adresse du NAS, par exemple « nas.lan », et
   collez la clé. Gardez HTTPS : TrueNAS révoque une clé qu'elle reçoit
   jamais en HTTP simple depuis une autre machine.

!!! warning

    Cette clé peut tout changer sur le NAS : TrueNAS n'offre pas de clé en lecture seule via son API REST. Traitez-la comme le mot de passe d'un compte qui peut effacer vos pools, gardez-la pour DumbMonit seul, et révoquez-la depuis la même page si elle fuit. TrueNAS livre aussi un certificat autosigné : si la connexion est refusée pour cette raison, cochez « Accepter un certificat non vérifiable » dans les options.

Adresse : `192.168.1.20`, `nas.lan`, `nas.lan:8443`, `[fd00::20]` ou une
URL complète. Le schéma `https` et le port 443 sont ajoutés s'ils
manquent.

TrueNAS a annoncé que l'API REST sera retirée dans TrueNAS 26, au profit
de l'API WebSocket. Jusque là, REST est ce que chaque release jusqu'à
25.10 sert.

## Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `scheme` | Protocole | `https` | Gardez HTTPS : TrueNAS révoque une clé d'API qu'elle reçoit en HTTP simple depuis une autre machine. |
| `port` | Port de l'interface web | `443` | Utilisé si l'adresse ne donne pas de port. |
| `insecure_tls` | Accepter un certificat non vérifiable | `false` | TrueNAS livre un certificat autosigné par défaut. |
| `request_timeout_seconds` | Délai par requête (secondes) | `20` | Temps accordé à chaque appel d'API, de 1 à 120. |
| `datasets` | Surveiller les datasets | `true` | Usage par rapport au quota, nombre de snapshots, dernier changement de snapshot. |
| `disks` | Surveiller les disques | `true` | Inventaire et températures en cache ; un disque endormi n'est jamais réveillé. |
| `smart` | Lire les résultats des tests SMART | `true` | Le dernier autotest de chaque disque ; aucun n'est jamais démarré. |
| `alerts` | Lire les propres alertes de TrueNAS | `true` | La liste d'alertes que TrueNAS garde lui-même, les alertes rejetées laissées de côté. |
| `tasks` | Surveiller les tâches de réplication et de snapshot | `true` | État, dernière exécution et dernier snapshot de chaque tâche. |
| `services` | Surveiller les services | `true` | Services réglés pour démarrer avec le NAS, et s'ils tournent. |

## Erreurs courantes

| Symptôme | Cause probable |
|---|---|
| Erreur d'authentification mentionnant Local Administrator | L'utilisateur de la clé n'est pas un administrateur complet — généralement un Read-Only Administrator. TrueNAS refuse une telle clé à chaque appel REST, avec un `403` vide. |
| Erreur d'authentification « Invalid API key » | La clé a été mal saisie, supprimée, expirée — ou révoquée par TrueNAS parce qu'elle a été envoyée une fois en HTTP simple depuis une autre machine. Créez-en une nouvelle et gardez HTTPS. |
| Erreur de certificat | Certificat autosigné : installez-en un de confiance (ACME est intégré) ou activez `insecure_tls`. |
| Pas de températures de disque | Le NAS tourne dans une machine virtuelle, ou TrueNAS n'a pas encore interrogé les disques depuis son démarrage. Les lectures viennent de son cache. |
| Pas de série SMART | Aucun autotest n'a jamais tourné sur ce disque, ou le NAS tourne sous TrueNAS 25.10, qui a retiré cet appel. Planifiez des tests SMART dans Data Protection. |
| Pas de `pool_last_scrub_age_seconds` | Le dernier scan de ce pool était un resilver, qui efface l'enregistrement du dernier scrub, ou le pool n'a jamais été scrubbé. |
| Un dataset n'a pas de série de quota | Il n'a pas de quota. Un quota non réglé n'est pas un quota de zéro. |
| « Pool NAS dégradé » et la page ne montre aucun nom de disque | Le pool ne peut pas être ouvert du tout (exporté, ou trop de disques manquants) : TrueNAS ne rapporte alors aucune disposition. |
| `scrape_errors` au-dessus de zéro | Un appel optionnel a échoué pour une raison autre qu'un chemin manquant — un délai dépassé, généralement. Les autres métriques ne sont pas affectées. |

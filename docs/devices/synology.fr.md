# Synology DSM

NAS Synology via l'API web DSM : volumes, pools de stockage, santé des
disques, température, tâches Hyper Backup, tâches Active Backup for
Business, et Synology Drive et Photos. La première raison de surveiller
un NAS est ses disques : un disque qui chauffe ou dont le statut SMART se
dégrade annonce une panne longtemps avant qu'un volume ne s'effondre.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_synology_`.

| Famille | Métriques | Labels |
|---|---|---|
| Système | `system_info` (modèle, version DSM, firmware…), `uptime_seconds`, `temperature_celsius`, `temperature_warning`, `system_crashed`, `system_need_repair`, `storage_health` | `model`, `dsm_version`, `firmware` |
| CPU et mémoire | `cpu_usage_percent`, `cpu_user/system/other_percent`, `cpu_cores`, `memory_total/used/available/installed_bytes`, `memory_usage_percent`, `swap_usage_percent` | |
| Réseau | `network_rx_bytes_per_second`, `network_tx_bytes_per_second` | `interface` |
| Volumes | `volume_status`, `volume_total/used/available_bytes`, `volume_used_percent`, `volume_used_warning_percent`, `volume_used_critical_percent` | `volume`, `fs_type`, `raid_type` |
| Pools de stockage et caches SSD | `pool_status`, `pool_failed_disks`, `pool_total/used_bytes`, et les mêmes quatre sous `ssd_cache_` | `pool` ou `ssd_cache`, `name`, `raid_type` |
| Disques | `disk_status`, `disk_smart_status`, `disk_temperature_celsius`, `disk_size_bytes`, `disk_bad_sector_exceeded`, `disk_life_below_threshold`, `disk_remaining_life_percent` (SSD seulement : la vie que le disque dit lui rester), `disk_unc_count` (le compteur de secteurs illisibles de DSM, celui qu'il compare à son seuil de secteurs défaillants), `disk_info` | `disk`, `name` ; sur `disk_info` : `model`, `serial`, `vendor`, `firmware`, `type`, `ssd` |
| Hyper Backup | `backup_tasks`, `backup_running`, `backup_last_result`, `backup_last_run_age_seconds`, `backup_last_success_age_seconds`, `backup_next_run_in_seconds` | par tâche |
| Synology Drive | `drive_up`, `drive_team_folders`, `drive_team_folder_info`, `drive_connections` (voir ci-dessous) | `drive_team_folder_info` : `name` |
| Synology Photos | `photos_installed` (voir ci-dessous) | |
| Réglages de sécurité | `update_available`, `admin_account_enabled`, `autoblock_enabled`, `firewall_enabled` (`0`/`1` ; lus toutes les heures, compte administrateur seulement, facultatifs : absents si DSM refuse) | |
| Collection | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Les métriques Active Backup for Business sont l'exception : elles sont
préfixées `dumbmonit_abb_` (voir ci-dessous).

`volume_status`, `disk_status` et `disk_smart_status` portent le propre
mot de DSM dans un label (`status`, `smart_status`) et une valeur sur
une échelle : `0` normal, `1` attention (un état transitoire comme
`repairing` ou `checking`, ou un mot que DSM a introduit et que
DumbMonit ne connaît pas encore), `2` critique (`degrade`, `crashed`,
`failed`…). L'inconnu compte délibérément comme `1`, jamais `0`.

Une panne partielle reste une sonde réussie : si l'inventaire de
stockage échoue, l'état système et l'utilisation sont quand même
publiés et l'échec est compté dans `scrape_errors`. Seul un échec sur
l'appel de découverte d'API ou sur la connexion condamne toute la sonde.

!!! note "Ce que les lignes de disque ne peuvent pas montrer"
    Les compteurs de secteurs réaffectés vivent dans le tableau
    d'attributs SMART par disque, que DSM ne sert que via un appel non
    documenté dont la forme n'a pas pu être confirmée par DumbMonit. Les
    lignes montrent donc ce que `load_info` donne : le verdict de
    secteurs défaillants de DSM, son compteur de secteurs illisibles (et
    sa croissance), et la vie restante du SSD.

!!! note "Ce qui change entre versions de DSM"
    Le même appel ne répond pas aux mêmes champs sur chaque DSM, et
    DumbMonit lit les deux formes plutôt que d'en supposer une :

    * `disk_bad_sector_exceeded` vient de `exceed_bad_sector_thr` jusqu'à
      DSM 7.1 et de `sb_days_left_critical` — l'estimation de DSM des
      jours restants avant que le disque n'atteigne son seuil de
      secteurs défaillants — sur DSM 7.2 et plus récent, où le premier
      champ a disparu. Pareil pour `disk_life_below_threshold` et
      `remain_life_danger` ;
    * `disk_remaining_life_percent` est un nombre simple sur les DSM
      plus anciens et un objet portant la valeur et si DSM lui fait
      confiance sur DSM 7.2 et plus récent. Une valeur que DSM marque
      non fiable n'est pas publiée ;
    * `volume_used_warning_percent` et `volume_used_critical_percent`
      répètent les seuils réglés dans DSM. DSM 7.4 ne les renvoie plus,
      donc sur un NAS récent ces deux séries n'existent simplement pas —
      rien n'est inventé à leur place.

## La page de la surveillance

![Une surveillance Synology : vue d'ensemble, volumes, disques et Active Backup](../assets/screenshots/device-synology-light.png){ loading=lazy }

Une surveillance Synology obtient son propre panneau au-dessus des
graphiques :

* **Vue d'ensemble** — CPU, mémoire (utilisée sur total), température et
  uptime, avec le propre verdict de stockage de DSM comme plaque (sain,
  attention, crashé, réparation nécessaire) ;
* **Volumes** — une ligne par volume : barre d'usage, utilisé sur total,
  type RAID et système de fichiers, l'état du volume dans le mot de DSM
  (normal, dégradé, crashé, en réparation…) ;
* **Pools de stockage** — une ligne par pool : type RAID, l'état du pool
  dans le mot de DSM, combien de ses disques DSM rapporte comme
  défaillants, et combien en est déjà donné aux volumes. C'est là que
  vit la redondance : un RAID 5 qui perd un disque garde le volume
  au-dessus à `normal` pendant qu'il se reconstruit, le volume ne dit
  donc rien et celui-ci dit tout. Un pool entièrement alloué à ses
  volumes est l'état final normal, pas un avertissement, ce chiffre est
  donc une phrase plutôt qu'une jauge colorée ;
* **Cache SSD** — une ligne par cache : son type RAID, son état et sa
  taille. La section est absente sur un NAS sans cache. Le `load_info`
  de DSM ne porte aucun mode de cache ni taux de succès, aucun des deux
  n'est donc montré ;
* **Disques** — une ligne par baie : modèle, taille, température,
  verdict SMART, verdict de secteurs défaillants de DSM, le compteur de
  secteurs illisibles et, sur un SSD, la vie restante comme une petite
  barre ;
* **Active Backup for Business** — les tâches, puis chaque appareil
  sauvegardé avec son rythme et une bande de trente jours (voir
  ci-dessous).

Tout vient de ce que la sonde a stocké : ouvrir la page n'interroge
jamais le NAS.

### Règles intégrées pour le NAS

| Règle | Gravité | Se déclenche quand |
|---|---|---|
| Avertissement SMART de disque Synology | avis | DSM ne rapporte plus le statut SMART du disque comme `normal`, depuis 15 minutes. |
| Disque Synology en échec | avertissement | DSM rapporte le disque comme crashé ou en échec, depuis 5 minutes. |
| Secteurs défaillants de disque Synology | avertissement | Le compteur de secteurs défaillants dépasse le seuil réglé dans DSM, depuis 15 minutes. |
| Secteurs défaillants de disque Synology en croissance | avis | De nouveaux secteurs illisibles sont apparus dans les dernières 24 heures (`delta(…disk_unc_count[24h]) > 0`). |
| SSD Synology qui s'use | avis | Un SSD a moins de 10 % de sa vie nominale restante, depuis une heure. |
| Volume Synology dégradé | avertissement | Un volume est dégradé ou crashé, depuis 5 minutes. |
| Pool de stockage Synology dégradé | avertissement | Un pool de stockage ou cache SSD est dégradé ou crashé, depuis 5 minutes. Un RAID 5 qui perd un disque se montre ici d'abord : DSM garde le volume au-dessus à `normal` pendant qu'il se reconstruit. |
| Volume Synology presque plein | avis | Un volume est plein à 90 % ou plus, depuis 15 minutes. |
| Température Synology élevée | avis | Un disque est au-dessus de 55 °C, ou DSM a levé son avertissement de température, depuis 15 minutes. |
| Mémoire Synology élevée | avis | Usage mémoire au-dessus de 95 % pendant 15 minutes. |

## Active Backup for Business

Active Backup for Business (ABB) sauvegarde PC, serveurs physiques,
machines virtuelles et serveurs de fichiers vers le NAS. Si le paquet
est installé et tourne, DumbMonit lit ses tâches à chaque sonde ; sinon,
le NAS n'annonce simplement pas l'API et rien n'est demandé. L'option
`abb` désactive toute la famille.

Pareil pour Hyper Backup : DSM ne liste les API d'un paquet que tant que
ce paquet tourne, un NAS dont Hyper Backup est arrêté — ou qui ne reçoit
des sauvegardes que via Hyper Backup Vault — ne publie donc aucune
métrique `backup_*` et ne rapporte non plus aucune erreur. DumbMonit
demande au NAS quelles API il a avant d'en appeler une, un paquet
manquant est donc du silence, jamais une sonde en échec.

| Métrique | Valeur | Labels |
|---|---|---|
| `dumbmonit_abb_tasks` | Nombre de tâches. | |
| `dumbmonit_abb_task_last_status` | Dernière exécution : `1` succès, `0` échec, `2` en cours, `-1` inconnu (jamais exécuté, annulé, pas de sauvegarde). Un succès *partiel* compte comme échec : au moins un appareil n'a pas été sauvegardé. | `task`, `task_id`, `source_type`, `result` |
| `dumbmonit_abb_task_last_success_seconds` | Ancienneté de la dernière exécution de sauvegarde réussie, en secondes. Absent pour une tâche qui n'a jamais réussi. | `task`, `task_id`, `source_type` |
| `dumbmonit_abb_task_enabled` | `1` si la tâche a un calendrier et n'est pas en pause, `0` si elle ne tourne qu'à la main. | `task`, `task_id`, `source_type` |
| `dumbmonit_abb_task_device_count` | Appareils attachés à la tâche. | `task`, `task_id`, `source_type` |

`task` est le nom de la tâche tel qu'affiché dans ABB ; `source_type`
est `pc`, `vm`, `physical_server`, `file_server` ou `nas` ; `result` est
le résultat brut de la dernière exécution (`success`, `partial_success`,
`fail`, `cancel`, `no_backup`, `running`, `none`).

L'ancienneté est prise depuis l'historique de tâche filtré sur les
exécutions de sauvegarde réussies, pas depuis le « dernier résultat » de
la tâche : après une purge de versions nocturne, le dernier résultat est
la purge, pas la sauvegarde. Ça coûte une requête par tâche, plafonnée à
vingt tâches.

Règles intégrées sur les tâches :

| Règle | Gravité | Se déclenche quand |
|---|---|---|
| Tâche Active Backup échouée | avertissement | La dernière exécution a échoué, ou était un succès partiel, depuis 10 minutes. La notification nomme la tâche. |
| Active Backup trop ancienne | avis | Aucune exécution réussie depuis plus de deux jours. |
| Tâche Active Backup désactivée | info | La tâche n'a pas de calendrier, ou sa sauvegarde continue est en pause, depuis une heure. |

### Appareils et leur rythme

Une règle au niveau tâche convient à un serveur et pas à un ordinateur
portable : les gens ne laissent pas leur portable allumé, et « pas de
sauvegarde depuis 24 heures » est lundi matin pour chaque PC de bureau.
DumbMonit surveille donc aussi ABB **appareil par appareil**, contre les
propres habitudes de l'appareil.

À chaque sonde, le collecteur lit les exécutions par appareil que ABB
garde (`SYNO.ActiveBackup.Overview` `list_device_transfer_size`, un
appel pour tous les appareils, seulement ce qui a changé depuis la
sonde précédente) et les copie dans sa propre base de données — bornée
à quatre-vingt-dix jours par NAS — pour que le modèle survive à un
redémarrage sans relire un mois d'historique. Puis, pour chaque
appareil, il regarde les trente derniers jours.

| Métrique | Valeur | Labels |
|---|---|---|
| `dumbmonit_abb_device_state` | `0` ok, `1` idle, `2` apprentissage, `3` en cours, `4` en retard, `5` en échec, `6` jamais. | `device`, `device_id`, `task`, `task_id`, `state` |
| `dumbmonit_abb_device_overdue` | `1` tant que l'appareil est en retard, `0` sinon. | `device`, `device_id`, `task`, `task_id` |
| `dumbmonit_abb_device_consecutive_failures` | Tentatives échouées à la suite depuis le dernier succès (une exécution annulée ne compte comme aucune des deux). | identiques |
| `dumbmonit_abb_device_last_success_seconds` | Ancienneté de la dernière exécution réussie de cet appareil. Absent s'il n'a jamais réussi. | identiques |
| `dumbmonit_abb_device_typical_interval_seconds` | Écart médian entre deux succès, jours de repos exclus. Absent pendant l'apprentissage. | identiques |

#### Comment « en retard » est jugé {#how-overdue-is-judged}

En mots simples, pour chaque appareil :

1. **Quels jours est-il actif ?** Un jour de la semaine où l'appareil a
   tenté au moins une sauvegarde dans les trente derniers jours —
   réussie ou non ; un échec prouve que la machine était allumée autant
   qu'un succès — est un *jour actif*. Une fois qu'il y a au moins deux
   semaines d'historique, un jour de la semaine sans aucune tentative
   est un *jour de repos*. Un portable utilisé du lundi au vendredi
   obtient samedi et dimanche comme jours de repos ; un serveur n'en a
   aucun.
2. **À quelle fréquence sauvegarde-t-il ?** Les écarts entre succès
   consécutifs sont rassemblés en *temps actif* : les jours de repos
   complets qu'un écart contient ne sont pas comptés (vendredi soir à
   lundi soir est un jour de temps actif, pas trois). Leur médiane est
   l'*intervalle typique* de l'appareil ; leur 90e percentile est à quoi
   ressemble « une semaine lente ».
3. **Combien de temps lui est accordé ?** La marge est
   `max(1,5 × écart p90, 2 × intervalle typique, 36 h)`. Un bureau
   nocturne obtient 48 h ; une machine sauvegardée tous les trois jours
   obtient environ six ; personne n'obtient moins d'un jour et demi.
   Tant qu'il y a moins de deux écarts utilisables (un appareil ajouté
   cette semaine), la marge est 72 h — assez pour traverser un week-end
   sans fausse alerte.
4. **Combien de temps actif s'est écoulé ?** Le temps depuis le dernier
   succès, moins les jours de repos complets qu'il contient. Un
   portable sauvegardé pour la dernière fois vendredi à 20h00 a, le
   lundi à 08h00, douze heures de temps actif derrière lui, pas
   soixante.
5. **Le verdict**, dans cet ordre : *en cours* (une sauvegarde est en
   cours) ; *en échec* (les deux dernières tentatives ou plus ont
   échoué — un succès partiel est un échec, une exécution annulée n'est
   aucun des deux) ; *jamais* (aucun succès enregistré) ; *en retard*
   (le temps actif dépasse la marge) ; *idle* (aujourd'hui est un de ses
   jours de repos : éteint, comme d'habitude) ; *apprentissage* (rythme
   pas encore connu) ; *ok*.

Seul *en retard* déclenche `dumbmonit_abb_device_overdue`, et seul *en
échec* alimente la règle d'échec. Un portable idle un dimanche ne
déclenche rien ; le même portable encore silencieux mardi soir si.

Jours et heures sont exprimés dans l'heure locale du serveur (`TZ` du
conteneur), celle où vit la personne qui lit la page. La page de
la surveillance montre, par appareil : l'état en un mot, le dernier succès et
dernier résultat, le rythme en mots (« jours de semaine, vers 20h00 »,
« tous les ~3 jours, lun, mer, ven, vers 03h00 », « encore en
apprentissage (2 exécutions en 30 jours) ») et une bande de trente
jours — teal pour un jour avec un succès, rouge pour un échec, ambre
pour une exécution annulée, gris pour aucune exécution.

Règles intégrées sur les appareils :

| Règle | Gravité | Se déclenche quand |
|---|---|---|
| Appareil Active Backup en retard | avis | `dumbmonit_abb_device_overdue > 0` depuis 30 minutes. |
| Appareil Active Backup en échec | avertissement | Deux tentatives échouées ou plus à la suite, depuis 10 minutes. |

!!! warning "Permissions"
    ABB ne répond qu'à un compte du groupe `administrators` avec le
    paquet autorisé dans ses permissions d'application, ou un compte à
    qui le paquet a été délégué (DSM 7 : Active Backup for Business →
    Settings → Privileges). Un compte à qui l'application est refusée
    ne peut pas lire les tâches ABB : l'appel est refusé (code 105) et
    compté dans `scrape_errors`. Soit autorisez soit déléguez le paquet
    au compte de surveillance, soit décochez « Surveiller Active Backup
    for Business » (`abb = false`) pour arrêter de demander.

!!! note "Une API non documentée"
    Synology ne documente pas l'API web ABB. DumbMonit suit la forme
    enregistrée par des projets communautaires (`SYNO.ActiveBackup.Task`
    `list`, `SYNO.ActiveBackup.Log` `list_result` et
    `SYNO.ActiveBackup.Overview` `list_device_transfer_size`, tous en
    version 1) et découvre leur chemin via `SYNO.API.Info`. Un DSM dont
    les réponses diffèrent produit des métriques manquantes, jamais une
    sonde échouée ; un ABB trop ancien pour offrir la vue par appareil
    garde ses métriques de tâche et n'a simplement pas de rythme
    d'appareil.

    La version 1 est délibérée. DSM 7.4 annonce ces API comme versions
    1 à 2, mais seule la version 1 porte les méthodes de lecture :
    demandées en version 2 elles répondent toutes « the requested
    method does not exist », ce qui ressemble à une différence de
    firmware et ne l'est pas. La même réponse revient aussi quand le
    jeton CSRF manque, DumbMonit en demande donc un à DSM à la connexion
    et l'envoie à chaque appel.

## Synology Drive

DSM ne documente pas l'API web Synology Drive. DumbMonit lit trois
appels dont le nom et la forme viennent de la bibliothèque
communautaire `N4S4/synology-api`, qui les appelle depuis les mêmes
écrans que la console d'administration Synology Drive, et suit la même
règle qu'Active Backup for Business : une API qu'un NAS n'annonce pas
est un paquet qui n'est pas installé, jamais une panne.

| Métrique | Valeur | Labels |
|---|---|---|
| `dumbmonit_synology_drive_up` | `1` tant que le service répond à `get_status`. Absent si le paquet n'est pas installé. | |
| `dumbmonit_synology_drive_team_folders` | Nombre de dossiers d'équipe. | |
| `dumbmonit_synology_drive_team_folder_info` | Valeur `1` par dossier d'équipe (32 au plus). | `name` |
| `dumbmonit_synology_drive_connections` | Nombre de connexions actives (clients bureau et mobiles actuellement synchronisés). | |

!!! note "Pourquoi les connexions ne sont qu'un compteur"
    La page « Liste des clients » de la console d'administration montre
    le nom d'appareil, l'utilisateur, le type d'application, le statut
    en ligne, l'adresse IP et la localisation de chaque connexion, mais
    c'est une description de la page, pas des champs JSON de l'API : ni
    Synology ni aucun projet communautaire ne documente ce que
    `SYNO.SynologyDrive.Connection` nomme réellement ses champs. Plutôt
    que de deviner le nom d'un appareil ou sa dernière date de sync
    depuis une clé non confirmée, DumbMonit ne publie que le compteur de
    connexions. Les appareils individuels rejoindront la même table
    [appareils clients](../alerting/rules.md) qu'utilisent Immich,
    Proxmox Backup Server et Veeam une fois cette forme confirmée depuis
    la réponse d'un vrai NAS.

L'option `drive` (activée par défaut) désactive toute la famille.

## Synology Photos

Synology Photos fournit une API de navigation bien documentée, mais ni
Synology ni les bibliothèques communautaires n'exposent une liste à
l'échelle administrateur des appareils de sauvegarde mobile ou de leur
dernière sauvegarde — l'application mobile montre son propre statut de
sauvegarde localement, et rien ne le centralise côté NAS via une API.
DumbMonit rapporte donc seulement que le paquet est installé :
`dumbmonit_synology_photos_installed` (valeur `1`), lu depuis le
catalogue d'API sans aucune requête supplémentaire au NAS — le même
catalogue `SYNO.API.Info` que répond déjà pour chaque autre paquet.
L'option `photos` (activée par défaut) le désactive.

## Ce qu'il faut préparer sur le NAS

Les étapes ci-dessous sont celles que montre l'avis à côté du
formulaire. Le principe : un compte réservé à la surveillance, dépouillé
de tout ce dont l'API de DSM n'a pas besoin — jamais le compte avec
lequel vous vous connectez.

1. Dans DSM, ouvrez Control Panel → User & Group → User et cliquez sur Create. Nommez le compte comme suit et donnez-lui un mot de passe long utilisé nulle part ailleurs.

    ```
    dumbmonit
    ```

2. Rejoignez des groupes : cochez administrators. DSM ne répond aux appels de stockage, volume et SMART de disque qu'à ce groupe ; sans lui, le NAS se montre vivant mais ne dit rien de ses disques. Les deux étapes suivantes reprennent tout le reste.
3. Assignez les permissions de dossier partagé : Aucun accès sur chaque dossier partagé. Assignez les permissions d'application : Refuser tout sauf DSM, plus Active Backup for Business si vous voulez lire ses tâches. Passez les pages de quota et de limite de vitesse.
4. La vérification en deux étapes doit rester désactivée pour ce compte : aucun moniteur automatisé ne peut taper un code à usage unique. Si Control Panel → Security → Account l'impose, restreignez la règle aux groupes dont cet utilisateur n'est pas membre, ou exemptez-le.
5. Les paquets de sauvegarde ne sont lus que tant qu'ils sont installés et tournent : DSM n'annonce pas du tout l'API d'un paquet arrêté, les tâches Hyper Backup deviennent alors silencieuses plutôt que d'échouer, et un NAS qui ne reçoit que des sauvegardes (Hyper Backup Vault) n'a pas de tâches propres à montrer. Active Backup for Business ne répond en plus qu'à un compte autorisé à l'utiliser (Active Backup for Business → Settings → Privileges) ; sinon décochez l'option « Surveiller Active Backup for Business » de DumbMonit pour arrêter de demander.
6. Dans DumbMonit, saisissez l'adresse du NAS (HTTPS, port 5001 par défaut), puis le nom d'utilisateur et le mot de passe de ce compte.

!!! warning
    Le groupe administrators est exigé par l'API de stockage de DSM, pas par DumbMonit. C'est pourquoi ce compte n'a aucun dossier partagé, aucune application et un mot de passe utilisé nulle part ailleurs : il peut lire le NAS, pas toucher vos fichiers.

!!! note "Quels droits donnent quelles métriques"
    L'API DSM n'a pas besoin des mêmes droits pour chaque appel. L'état
    système (modèle, version, température, uptime) répond à tout
    compte. CPU, mémoire, volumes et, surtout, statut SMART des disques
    demandent le groupe `administrators`. Sur DSM 7, la délégation
    « System monitoring » (Control Panel → User & Group → Delegation)
    suffit pour l'utilisation mais pas pour l'inventaire de stockage. Un
    compte sans droits particuliers donne donc un NAS « vivant », mais
    rien sur ses disques.

Documentation du fournisseur : [Create a user in DSM](https://kb.synology.com/en-global/DSM/help/DSM/AdminCenter/file_user_create).

## Identifiants

| Identifiant | Champs |
|---|---|
| Compte DSM | **Nom d'utilisateur** : le compte créé ci-dessus. **Mot de passe** : son mot de passe. La vérification en deux étapes doit être désactivée pour ce compte : aucun moniteur automatisé ne peut taper un code à usage unique. |

Adresse : `192.168.1.30`, `nas.lan` ou `nas.lan:5001`. Sans port, 5001
est utilisé en HTTPS et 5000 en HTTP.

## Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `scheme` | Protocole | `https` | HTTPS convient à un NAS neuf dès le départ. HTTP seulement si DSM n'écoute qu'en texte clair. Choix : `https`, `http`. |
| `port` | Port DSM | *(vide)* | Utilisé si l'adresse ne donne pas de port. Vide : 5001 en HTTPS, 5000 en HTTP. |
| `insecure_tls` | Accepter un certificat non vérifiable | `false` | Un NAS livre un certificat autosigné par défaut : activez ceci si la connexion est refusée pour cette raison. |
| `request_timeout_seconds` | Délai par requête (secondes) | `15` | Temps accordé à chaque appel d'API, de 1 à 120. L'inventaire de stockage peut réveiller des disques endormis. |
| `abb` | Surveiller Active Backup for Business | `true` | Lit les tâches Active Backup for Business. A besoin du paquet installé et d'un compte autorisé à l'utiliser ; un NAS sans le paquet est simplement ignoré. |
| `drive` | Surveiller Synology Drive | `true` | Lit si le service Synology Drive répond, ses dossiers d'équipe et son compteur de connexions actives. Un NAS sans le paquet est simplement ignoré. |
| `photos` | Surveiller Synology Photos | `true` | Montre si le paquet Synology Photos est installé, sans requête supplémentaire au NAS. Synology n'expose aucune liste d'appareils de sauvegarde mobile via une API. |

## Erreurs courantes

| Symptôme | Cause probable |
|---|---|
| Erreur d'authentification | Mauvais mot de passe, ou vérification en deux étapes exigée pour ce compte. DSM répond HTTP 200 même en cas d'échec ; le code d'erreur dans le corps JSON est ce qui compte, et DumbMonit le lit. |
| Erreur de certificat | Certificat autosigné : activez `insecure_tls`. C'est une erreur de configuration, jamais « inaccessible » : un NAS qui répond n'est pas montré comme éteint. |
| Aucune métrique de disque ou de volume | Le compte n'est pas dans le groupe `administrators` (voir la note ci-dessus). |
| Aucune métrique `dumbmonit_abb_*`, `scrape_errors` à 1 | Le compte ne peut pas utiliser Active Backup for Business : déléguez-lui le paquet, ou désactivez l'option `abb`. Sans le paquet installé il n'y a ni erreur ni métrique. |
| Métriques de tâche mais pas de lignes d'appareil sur la page | Le NAS n'annonce pas `SYNO.ActiveBackup.Overview` (ABB plus ancien), ou la surveillance est sondée via un agent relais, qui garde l'historique en mémoire de son côté : les métriques existent, le calendrier non. |
| Un portable montre « En retard » le jour après son ajout | Moins de trois succès : le modèle apprend encore et autorise 72 heures entre sauvegardes ; ça se stabilise une fois deux semaines d'habitudes connues. |
| Les disques se réveillent à chaque sonde | L'inventaire de stockage réveille les disques endormis. Allongez l'intervalle de vérification. |

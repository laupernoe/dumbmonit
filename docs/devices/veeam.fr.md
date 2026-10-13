# Veeam Backup & Replication

Un serveur Veeam Backup & Replication via son API REST : jobs dont la
dernière exécution a échoué ou s'est terminée avec un avertissement,
sessions en échec des dernières 24 heures, dépôts qui se remplissent, la
licence, et — par ordinateur ou VM protégé — sa dernière sauvegarde
réussie.

DumbMonit se connecte avec un compte Windows qui détient seulement le rôle
Veeam Backup Viewer (OAuth2 password grant sur `/api/oauth2/token`, port
9419) et garde le jeton pour sa durée de vie de quinze minutes. Chaque
appel porte l'en-tête obligatoire `x-api-version`, `1.1-rev0` par défaut :
la révision qui est arrivée avec la version 12.0.

Les états de job (`/api/v1/jobs/states`) sont la seule lecture requise. Les
autres — `/api/v1/serverInfo`, `/api/v1/sessions` (créées dans les
dernières 24 heures), `/api/v1/backupInfrastructure/repositories/states`,
`/api/v1/license` et `/api/v1/restorePoints` (les points de restauration de
chaque objet protégé, à l'échelle du serveur, les plus récents d'abord) —
sont lues quand le rôle les permet. Veeam réserve certaines routes au rôle
Backup Administrator, la licence parmi elles sur les versions actuelles :
un refus là est montré comme « non lisible », jamais comme une erreur. Un
serveur trop ancien pour connaître `/api/v1/restorePoints` — la route est
apparue après 12.0 — est traité de la même façon : pas d'appareil, pas
d'erreur.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la référence de l'API
    REST de Veeam et du centre d'aide (authentification, versionnage, les
    modèles de job, session, dépôt, licence et point de restauration), pas
    contre un serveur de sauvegarde en fonctionnement. Les routes que le
    rôle Viewer peut lire varient selon la version : dites-nous ce que
    votre serveur refuse.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_veeam_`.

| Métrique | Labels | Signification |
|---|---|---|
| `version_info` | `version` | Valeur 1 : le build, `12.3.1.1139`. |
| `job_enabled`, `job_running` | `job`, `type` | Par job. |
| `job_failed`, `job_warning` | `job`, `type` | 1 quand la dernière exécution d'un job activé a échoué, ou s'est terminée avec un avertissement. Un job désactivé garde son dernier résultat, qui ne dit plus rien : il ne compte pour aucun des deux. |
| `job_result_info` | `job`, `type`, `result` | Valeur 1 : `success`, `warning`, `failed`, `none`. |
| `job_last_run_age_seconds` | `job`, `type` | Depuis la dernière exécution ; absent pour un job qui n'a jamais tourné. |
| `job_progress_percent` | `job`, `type` | Pendant que le job tourne. |
| `jobs`, `jobs_failed`, `jobs_warning`, `jobs_running`, `jobs_disabled` | | Compteurs ; 200 jobs au plus sont décrits un par un. |
| `sessions_24h`, `sessions_failed_24h`, `sessions_warning_24h` | | Sessions créées dans les dernières 24 heures, par résultat. |
| `repository_capacity_bytes`, `repository_free_bytes`, `repository_used_bytes`, `repository_used_percent` | `repository`, `type` | Par dépôt. Un dépôt de stockage objet n'a pas de capacité : seul son espace utilisé est publié. |
| `license_valid` | | 1 quand le statut de licence est `Valid`. |
| `license_info` | `status`, `type`, `edition` | Valeur 1. |
| `license_expiry_seconds`, `support_expiry_seconds` | | Jusqu'à l'expiration de la licence et du contrat de support ; absent pour une licence perpétuelle. |
| `license_instances_licensed`, `license_instances_used` | | Usage de licence par instance. |
| `section_readable` | `section` : `sessions`, `repositories`, `license`, `restore_points` | 1 quand le compte peut la lire, 0 quand Veeam a refusé. |

Les [règles intégrées](../alerting/rules.md#veeam-backup-replication) qui
s'appliquent :

- **Job Veeam en échec** : la dernière exécution d'un job activé a échoué
  (Avertissement, rappelé chaque jour).
- **Job Veeam terminé avec un avertissement** (Avis, rappelé chaque jour).
- **Dépôt Veeam presque plein** : plus de 90 % pendant trente minutes
  (Avis).
- **Licence Veeam arrivant à expiration** : moins de 30 jours restants
  (Avis, rappelé chaque jour). Seulement quand le compte peut lire la
  licence.
- **Appareil client périmé** : un ordinateur ou une VM protégé a dépassé
  `device_stale_days` sans sauvegarde réussie (Avis, rappelé chaque jour) ;
  voir [Appareils](#devices) ci-dessous.

## Appareils {#devices}

Chaque objet avec au moins un point de restauration — une VM, un
ordinateur physique ou virtuel protégé par agent, un partage de fichiers —
est publié sous la famille générique partagée par chaque intégration
DumbMonit qui suit les appareils protégés —
`dumbmonit_client_device_*`, préfixée sans `veeam`, documentée en entier
sous [Appareils clients](../alerting/rules.md#client-devices) :

| Métrique | Quoi | Labels |
|---|---|---|
| `last_backup_timestamp_seconds` | Le point de restauration le plus récent de l'objet | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positif une fois qu'un objet a dépassé `device_stale_days` (3 par défaut) sans nouveau point de restauration ; alimente la règle [Appareil client périmé](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (toujours `backup`) |

**Limites à connaître** : un job protège ses objets en groupe
(`jobs/states` donne seulement un compteur, `objectsCount`) ; la date par
objet vient de `/api/v1/restorePoints` à la place, lue pour tout le serveur
en un appel. Veeam ne crée un point de restauration qu'une fois une
sauvegarde au moins partiellement réussie, donc le plus récent par objet
est déjà « la dernière sauvegarde réussie » — rien n'est filtré sur le
résultat ici. Veeam n'expose pas de « dernière connexion » séparée de la
sauvegarde elle-même, donc `last_seen_timestamp_seconds` et le signal
`connection` ne sont jamais publiés ; un appareil montre
`last_connection` comme « — » dans le tableau de la page de l'appareil. Le
`type` de l'appareil est le propre nom de plateforme de Veeam (`VSphere`,
`WindowsPhysical`, `HyperV`…), montré tel quel.

## La page de l'appareil

Le panneau Backups dit ce qui ne va pas en une phrase, montre les jobs en
échec, les sessions en échec des dernières 24 heures, les jobs en cours et
la licence, puis liste les jobs activés — les échecs d'abord — avec leur
type et leur dernière exécution, et les dépôts avec leur remplissage et
leur espace libre. Le tableau des appareils protégés (nom, plateforme,
dernière sauvegarde, statut) suit, depuis la famille générique ci-dessus.

## Donner à DumbMonit un compte Veeam Backup Viewer

1. Sur le serveur de sauvegarde, créez un compte Windows local pour DumbMonit avec un mot de passe long et aléatoire et aucun autre droit sur la machine (un compte de domaine fonctionne aussi).

    ```
    net user dumbmonit * /add
    ```

2. Dans la console Veeam, ouvrez le menu principal > Users and Roles > Security, cliquez sur Add, saisissez le compte (par exemple VBR01\dumbmonit) et donnez-lui seulement le rôle Veeam Backup Viewer.

3. Dans DumbMonit, saisissez l'adresse du serveur de sauvegarde, par exemple « vbr.lan », et le compte comme HOST\dumbmonit ou DOMAIN\dumbmonit avec son mot de passe. L'API REST écoute sur le port 9419 avec un certificat autosigné : cochez Accepter un certificat non vérifiable sauf si vous avez installé le vôtre.

!!! warning
    Veeam réserve certaines routes d'API REST au rôle Backup Administrator, la licence parmi elles sur les versions actuelles. Avec le rôle Viewer, DumbMonit lit les jobs, sessions, dépôts et points de restauration, et montre la licence comme non lisible : il n'en demande jamais plus.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur et mot de passe | Le compte avec le rôle Veeam Backup Viewer, comme `HOST\dumbmonit` ou `DOMAIN\dumbmonit`. |

Adresse : le nom d'hôte ou l'IP du serveur de sauvegarde, `host:port`, ou
une URL. Options :

- **Protocole** (`https`) et **Port** (9419) : ceux du service d'API REST.
- **Version d'API** (`1.1-rev0`) : envoyée comme `x-api-version`.
  Changez-la seulement si le serveur la refuse ; un serveur v13 documente
  `1.3-rev1`.
- **Accepter un certificat non vérifiable** : le propre certificat de
  l'API REST est autosigné.
- **Délai par requête** (15 s).
- **Péremption d'appareil** (`device_stale_days`, 3 jours par défaut) :
  voir [Appareils](#devices) ci-dessus.

## Dépannage

« Veeam refused the login » : le nom d'utilisateur a besoin de son hôte ou
domaine (`VBR01\dumbmonit`), ou le compte n'a aucun rôle Veeam.

« Veeam refused to list the jobs (403) » : le compte a un rôle qui ne peut
pas lire les jobs ; donnez-lui Veeam Backup Viewer.

« Veeam refused the API version » : réglez l'option de version d'API sur
une que le serveur liste sur `https://<server>:9419/swagger`.

La licence montre « Not readable » : attendu avec le rôle Viewer sur les
versions actuelles ; DumbMonit n'en demande pas plus.

Aucun appareil, ou le panneau Appareils est vide : `restore_points` lit
« Not readable », probablement parce que le serveur est plus ancien que la
route (tout avant les révisions ultérieures de 12.0) plutôt qu'un problème
de rôle — les points de restauration sont un inventaire ordinaire, pas une
page réservée aux administrateurs.

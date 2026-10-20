# Immich

Si Immich tient le rythme de ce qui est envoyé : files de tâches qui se
bloquent ou restent en pause, mises à jour, le disque qui contient la
bibliothèque, combien de photos et vidéos elle contient, et — pour chaque
téléphone, tablette et ordinateur connecté au compte — sa dernière connexion
et sa dernière sauvegarde.

DumbMonit lit l'API d'Immich avec une **clé d'API limitée à huit permissions
de lecture**. La clé ne peut voir aucune photo, aucun album ni aucune
personne, et ne peut rien changer.

La panne à surveiller : **des tâches qui ne tournent plus**. Une file mise
en pause pendant un gros import et oubliée, ou un worker de tâches qui s'est
arrêté, et les nouvelles photos n'ont pas de vignette, pas de métadonnées,
pas de visage et pas de recherche intelligente, tandis que les envois
continuent de réussir. Immich ne le montre que sur sa page Jobs.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_immich_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `update_available` | 1 quand la propre vérification de version d'Immich connaît une release plus récente (absent quand la vérification est désactivée) | `latest_version` |
| `maintenance_mode` | 1 tant qu'Immich est en mode maintenance | |
| `storage_used_percent`, `storage_available_bytes`, `storage_size_bytes` | le disque qui contient la bibliothèque | |
| `photos`, `videos`, `usage_bytes`, `users` | taille de la bibliothèque, tous utilisateurs ensemble | |
| `jobs_waiting`, `jobs_active`, `jobs_failed`, `queues_paused` | toutes les files ensemble | |
| `queue_waiting`, `queue_active`, `queue_failed`, `queue_paused` | par file : en attente (y compris retardées et bloquées par une pause), en cours, en échec, en pause | `queue` |
| `statistics_readable` | 0 quand la clé appartient à un compte ordinaire et que les compteurs et files sont ignorés | |
| `scrape_errors` | appels qui ont échoué à la dernière sonde | |

Les versions récentes d'Immich retirent les tâches en échec de la file une
fois journalisées, donc `queue_failed` lit généralement 0 ; une tâche en
échec se montre dans le propre journal d'Immich.

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui
s'appliquent : tâches Immich bloquées, file Immich en pause, stockage
Immich presque plein, mise à jour Immich disponible, plus Appareil
inaccessible.

## Appareils

Chaque appareil connecté au compte qui possède la clé, avec sa dernière
connexion et sa dernière sauvegarde, est publié sous une famille générique
partagée par chaque intégration DumbMonit qui suit les appareils clients —
`dumbmonit_client_device_*`, préfixée sans `immich`, documentée en entier
sous [Appareils clients](../alerting/rules.md#client-devices) :

| Métrique | Quoi | Labels |
|---|---|---|
| `last_seen_timestamp_seconds` | Dernière connexion (dernière activité de la session) | `device`, `type`, `os`, `user`, `kind` |
| `last_backup_timestamp_seconds` | Dernière sauvegarde ; voir la limite ci-dessous | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positif une fois qu'un appareil a dépassé `device_stale_days` (3 par défaut) sans se connecter ni sauvegarder ; alimente la règle [Appareil client périmé](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (`connection` ou `backup`) |

**Limite à connaître** : l'API d'Immich n'enregistre pas quel appareil a
envoyé une photo donnée, donc `last_backup_timestamp_seconds` est l'envoi le
plus récent de toute la bibliothèque — la même valeur pour chaque appareil,
publiée sous un appareil nommé `library` plutôt qu'un vrai. Cela attrape
chaque appareil qui s'arrête d'un coup (stockage plein, le compte déconnecté
partout) ; cela n'attrapera pas un seul appareil qui devient silencieux
pendant qu'un autre continue d'envoyer. Seul
`last_seen_timestamp_seconds` — la dernière fois que la session de chaque
appareil était active — est vraiment par appareil.

Le tableau de la page de la surveillance montre ces chiffres pour chaque appareil,
avec un statut OK ou Périmé.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'appelle jamais Immich. Il dit d'abord ce qui ne va pas (files
bloquées, files en pause, maintenance), puis les mises à jour, puis les
photos, vidéos, taille de la bibliothèque, disque utilisé et tâches en
attente et en cours, puis les files qui ont du travail, par nom, puis le
tableau des appareils (nom, type, OS, dernière connexion, dernière
sauvegarde, statut).

## Créer une clé d'API en lecture seule dans Immich

1. Dans Immich, ouvrez Account Settings → API Keys → New API Key et nommez-la comme suit.

    ```
    dumbmonit
    ```

2. Cochez les droits listés ci-dessous, créez la clé et copiez-la : Immich ne la montre qu'une fois.

3. Dans DumbMonit, saisissez l'adresse d'Immich et collez la clé.

### Droits requis

- **server.about** : Version
- **server.versionCheck** : Mise à jour disponible
- **server.storage** : Espace disque
- **server.statistics** : Nombre de photos et de vidéos
- **queue.read** : Files de tâches (job.read sur un Immich ancien)
- **session.read** : Appareils et dernière connexion
- **asset.read** : Dernière sauvegarde
- **user.read** : Le compte propriétaire de la clé

Documentation de l'éditeur : <https://immich.app/docs/api/>

### Pourquoi ces droits

La clé lit ces huit pages et rien d'autre : aucune photo, aucun album, aucun réglage. Une clé créée par un compte ordinaire fonctionne aussi : la version, les mises à jour, l'espace disque et les appareils sont lus, les décomptes de photos et les files de tâches sont ignorés. Si votre Immich ne propose pas `queue.read`, cochez `job.read` à la place.

Une file en pause s'oublie facilement : les nouvelles photos n'ont ni miniature, ni métadonnées, ni visages, et Immich ne le dit que sur sa page Jobs. DumbMonit prévient quand des tâches attendent dans une file qui n'est pas en pause et ne fait rien tourner.

`session.read`, `asset.read` et `user.read` listent tous les appareils du compte propriétaire de la clé (ses téléphones, tablettes et navigateurs) avec la dernière connexion de chacun. L'API d'Immich n'enregistre pas quel appareil a envoyé une photo donnée : la « dernière sauvegarde » est donc le dernier envoi de toute la bibliothèque, affiché identique pour tous les appareils. Si un seul appareil cesse de sauvegarder alors que les autres continuent, sa connexion apparaîtra périmée mais la sauvegarde partagée paraîtra toujours fraîche.

!!! warning
    Créez la clé depuis un compte qui peut ouvrir Administration, sinon les décomptes de photos et les files de tâches sont ignorés ; la « dernière sauvegarde » est le dernier envoi de toute la bibliothèque, identique pour chaque appareil.

Ce que chaque permission ouvre : `server.about` la version
(`/api/server/about`), `server.versionCheck` la dernière release connue
d'Immich, `server.storage` le disque, `server.statistics` les compteurs de
photos et vidéos, `queue.read` les files de tâches (`/api/queues` ; les
versions plus anciennes répondent sur `/api/jobs` avec `job.read`),
`session.read` les appareils du compte (`/api/sessions`), `asset.read`
l'envoi le plus récent de la bibliothèque (`/api/search/metadata`),
`user.read` le nom d'affichage du compte (`/api/users/me`).

## Identifiants

| Identifiant | Champs |
|---|---|
| Clé d'API | Envoyée dans l'en-tête `x-api-key`. |

Adresse : un nom d'hôte (`immich.lan`), `host:port`, ou une URL complète avec
un préfixe de chemin ; un `/api` final est accepté. Options : protocole
(HTTP par défaut), port (2283), vérification du certificat, délai de requête
(15 s), et péremption d'appareil (`device_stale_days`, 3 jours par défaut).

Validé avec Immich v3.2.4 (images officielles, apprentissage machine
désactivé) ; les points de terminaison d'appareil ont été vérifiés contre le
tag `v3.2.4` de la propre spécification OpenAPI d'Immich (`immich-app/immich`),
qui a confirmé que `/api/search/metadata` n'accepte plus de filtre
`deviceId` — la raison pour laquelle « dernière sauvegarde » est à l'échelle
de la bibliothèque plutôt que par appareil.

# Jellyfin

Si Jellyfin tient ses bibliothèques à jour et sert bien ses spectateurs :
tâches planifiées qui ont échoué à leur dernière exécution (le scan de
bibliothèque en premier), extensions qui ne se chargent plus, un redémarrage
en attente, et ce qui joue en ce moment, transcodé ou non.

DumbMonit lit l'API de Jellyfin avec une **clé d'API** créée dans le tableau
de bord. Il ne démarre jamais de tâche, n'arrête jamais une lecture et ne
change jamais un réglage.

La panne à surveiller : **le scan de bibliothèque qui échoue chaque nuit**. Un
partage réseau démonté, un disque plein, et les nouveaux films et épisodes
cessent d'apparaître ; Jellyfin continue de jouer ce qu'il connaît déjà, et la
seule trace est une ligne dans les tâches planifiées.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_jellyfin_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `pending_restart` | 1 quand Jellyfin attend un redémarrage (après l'installation ou la mise à jour d'une extension) | |
| `shutting_down` | 1 pendant l'arrêt de Jellyfin | |
| `scheduled_tasks_failed`, `scheduled_tasks_running` | tâches dont la dernière exécution a échoué ou a été interrompue ; tâches en cours | |
| `scheduled_task_failed` | valeur 1 par tâche en échec (32 au plus) | `task`, `error` (première ligne du message) |
| `library_scan_last_ok`, `library_scan_age_seconds` | dernière exécution de « Scan Media Library » : réussie ou non, et depuis quand elle est terminée | |
| `plugins`, `plugins_broken` | extensions installées ; celles qui n'ont pas pu se charger ou ne supportent pas cette version | |
| `plugin_broken` | valeur 1 par extension cassée | `plugin`, `status` |
| `sessions`, `streams`, `streams_paused`, `transcodes` | sessions clientes ; celles qui jouent quelque chose, en pause, et transcodées | |
| `scrape_errors` | appels qui ont échoué à la dernière sonde | |

Jellyfin ne vérifie pas ses propres mises à jour (c'est le paquet ou l'image
qui le fait), il n'y a donc pas de métrique de mise à jour.

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui
s'appliquent : échec d'une tâche planifiée Jellyfin, extension Jellyfin
cassée, redémarrage Jellyfin en attente, plus Surveillance inaccessible.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'appelle jamais Jellyfin. Il dit d'abord ce qui ne va pas (tâches en
échec, extensions cassées, un redémarrage en attente), puis les flux, les
transcodages, les sessions et l'ancienneté du dernier scan de bibliothèque,
puis les tâches en échec et les extensions cassées par leur nom avec l'erreur
propre de Jellyfin.

## Créer une clé d'API dans Jellyfin

1. Dans Jellyfin : Dashboard → API Keys (sous Advanced) → +, et nommez la clé comme suit. Copiez-la.

    ```
    DumbMonit
    ```

2. Jellyfin ne peut pas limiter ce qu'une clé peut faire : chaque clé a tous les droits sur le serveur. C'est pourtant la seule façon de lire les tâches planifiées et la lecture de tout le monde ; un utilisateur ordinaire ne voit ni l'une ni l'autre.

3. Dans DumbMonit, saisissez l'adresse de Jellyfin, par exemple « jellyfin.lan » (port 8096) ou « https://media.example.com » derrière un reverse proxy, et collez la clé.

4. DumbMonit ne lit que /System/Info, /ScheduledTasks, /Sessions et /Plugins. Il ne démarre jamais de tâche, n'arrête jamais une lecture et ne change jamais un réglage.

!!! warning
    Comme la clé porte tous les droits, gardez-la sur un réseau de confiance et mettez Jellyfin derrière HTTPS sur l'internet public. Si elle fuit un jour, révoquez-la sur la même page : rien d'autre ne l'utilise.

## Identifiants

| Identifiant | Champs |
|---|---|
| Clé d'API | Envoyée dans l'en-tête `Authorization: MediaBrowser … Token="…"`, la forme que Jellyfin 10.11 et plus récent acceptent par défaut. |

Adresse : un nom d'hôte (`jellyfin.lan`), `host:port`, ou une URL complète avec
un préfixe de chemin (`https://example.com/jellyfin`) ; un `/web` final est
accepté. Options : protocole (HTTP par défaut), port (8096), vérification du
certificat et délai de requête (15 s).

Validé avec Jellyfin 12.1.0 (image officielle).

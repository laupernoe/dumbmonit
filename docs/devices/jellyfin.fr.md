# Jellyfin

Si Jellyfin tient ses bibliothèques à jour et sert bien ses spectateurs :
tâches planifiées qui ont échoué à leur dernière exécution (le scan de
bibliothèque en premier), extensions qui ne se chargent plus, un redémarrage
en attente, et ce qui joue en ce moment, transcodé ou non.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 8096 | TCP | API (HTTP, par défaut) |
| 8920 | TCP | Seulement avec le schéma https sur le port HTTPS propre à Jellyfin |

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
| `library_scan_scheduled` | 1 quand « Scan Media Library » a un déclencheur périodique (pas seulement au démarrage) ; évite que la règle de scan en retard vise un scan que vous avez désactivé | |
| `libraries`, `libraries_scanning` | bibliothèques ; celles que Jellyfin actualise en ce moment | |
| `sessions`, `streams`, `streams_paused`, `transcodes` | sessions clientes ; celles qui jouent quelque chose, en pause, et transcodées | |
| `transcodes_software` | transcodages sans accélération matérielle, donc sur le processeur | |
| `streaming_users` | utilisateurs distincts qui lisent quelque chose | |
| `users`, `users_disabled` | comptes ; ceux désactivés | |
| `failed_logins_recent` | connexions refusées dans la dernière heure, lues dans le journal d'activité (500 entrées au plus) | |
| `storage_used_percent`, `storage_free_bytes` | une série par disque (bibliothèques, données, métadonnées, cache, transcodages, journaux ; les dossiers d'un même disque comptent une fois) ; Jellyfin 10.11 et plus | `volume` |
| `scrape_errors` | appels qui ont échoué à la dernière sonde | |

Jellyfin ne vérifie pas ses propres mises à jour (c'est le paquet ou l'image
qui le fait), il n'y a donc pas de métrique de mise à jour.

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui
s'appliquent : échec d'une tâche planifiée Jellyfin, extension Jellyfin
cassée, redémarrage Jellyfin en attente, scan de bibliothèque Jellyfin en
retard (aucun scan terminé depuis trois jours, seulement si le scan est
planifié), stockage Jellyfin presque plein (90 %), transcodage Jellyfin lourd
sur processeur (trois transcodages logiciels ou plus pendant 15 minutes),
connexions Jellyfin refusées (dix en une heure), plus Surveillance
inaccessible.

Tout ce qui suit les quatre premiers appels (stockage, bibliothèques, comptes,
journal d'activité) est facultatif : un Jellyfin qui ne le propose pas (le
stockage demande la 10.11) ou une clé qui ne peut pas le lire est sauté sans
erreur.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'appelle jamais Jellyfin. Il dit d'abord ce qui ne va pas (tâches en
échec, extensions cassées, un redémarrage en attente), puis les flux, les
transcodages, les sessions et l'ancienneté du dernier scan de bibliothèque,
puis les tâches en échec et les extensions cassées par leur nom avec l'erreur
propre de Jellyfin.

## Créer une clé d'API dans Jellyfin

1. Dans Jellyfin, ouvrez Dashboard → Advanced → API Keys → + et nommez la clé comme suit, puis copiez-la.

    ```
    DumbMonit
    ```

2. Dans DumbMonit, saisissez l'adresse de Jellyfin et collez la clé.

Documentation de l'éditeur : <https://api.jellyfin.org/>

### Pourquoi une clé aux pleins droits

Jellyfin ne peut pas limiter ce qu'une clé a le droit de faire : toute clé a tous les droits sur le serveur. C'est pourtant le seul moyen de lire les tâches planifiées et les lectures de tout le monde ; un utilisateur ordinaire ne voit ni les unes ni les autres.

DumbMonit lit seulement `/System/Info`, `/ScheduledTasks`, `/Sessions`, `/Plugins`, `/System/Info/Storage`, `/Library/VirtualFolders`, `/Users` et `/System/ActivityLog/Entries`. Il ne lance jamais une tâche, n'arrête jamais une lecture et ne change aucun réglage. Gardez la clé sur un réseau de confiance et placez Jellyfin derrière HTTPS à travers Internet.

!!! warning
    Les clés Jellyfin ne peuvent pas être limitées : celle-ci a tous les droits sur le serveur, gardez-la pour DumbMonit seul et révoquez-la sur la même page si elle fuit.

## Dépannage

- **Aucun chiffre de stockage, de bibliothèque ou d'utilisateur** : le stockage demande Jellyfin 10.11 ou plus, et ces appels sont sautés (sans erreur) quand la clé ne peut pas les lire. Un appel qui échoue vraiment incrémente `scrape_errors`.
- **« Scan de bibliothèque en retard » ne sonne jamais** : elle ne surveille qu'un scan qui a un déclencheur périodique ; vérifiez Dashboard → Scheduled Tasks → Scan Media Library.
- **Une tâche échoue sans cesse** : le label `error` de `scheduled_task_failed` porte la première ligne du message de Jellyfin ; la trace complète est dans le journal de Jellyfin.
- **Les connexions refusées restent à 0** : elles viennent du journal d'activité, que la clé doit pouvoir lire ; le chiffre ne couvre que la dernière heure.
- **Non autorisé** : la clé a été révoquée ou mal saisie ; créez-en une autre dans Dashboard → Advanced → API Keys.

## Identifiants

| Identifiant | Champs |
|---|---|
| Clé d'API | Envoyée dans l'en-tête `Authorization: MediaBrowser … Token="…"`, la forme que Jellyfin 10.11 et plus récent acceptent par défaut. |

Adresse : un nom d'hôte (`jellyfin.lan`), `host:port`, ou une URL complète avec
un préfixe de chemin (`https://example.com/jellyfin`) ; un `/web` final est
accepté. Options : protocole (HTTP par défaut), port (8096), vérification du
certificat et délai de requête (15 s).

Validé avec Jellyfin 12.1.0 (image officielle).

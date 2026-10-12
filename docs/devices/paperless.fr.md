# Paperless-ngx

Si Paperless-ngx importe toujours ce qu'on lui envoie : la santé de sa base
de données, de Redis, des workers Celery, de l'index de recherche, du
classificateur et du contrôle de cohérence, les tâches en échec, les mises à
jour, l'espace disque et le nombre de documents.

DumbMonit lit l'API REST de Paperless-ngx avec un **utilisateur dédié qui ne
peut lire aucun document** : trois permissions de lecture lui donnent la liste
des tâches, l'état système et les compteurs globaux, rien d'autre.

La panne à surveiller : **Redis ou Celery arrêté**. L'interface web s'ouvre
et les documents peuvent être lus, mais rien de ce qui est déposé dans le
dossier de consommation ou récupéré par mail n'est importé jusqu'à ce que
quelqu'un remarque la pile. L'état système le dit ; DumbMonit alerte dessus.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_paperless_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `update_available` | 1 quand la propre vérification de version de Paperless-ngx connaît une release plus récente (absent quand la vérification est désactivée) | `latest_version` |
| `component_status` | 0 OK, 1 avertissement, 2 erreur, comme la page d'état système le rapporte | `component` : `database`, `redis`, `celery`, `index`, `classifier`, `sanity_check`, `llm_index` (seulement si activé) |
| `unapplied_migrations` | migrations de base de données pas encore appliquées | |
| `storage_available_bytes`, `storage_total_bytes`, `storage_used_percent` | le disque qui contient les médias | |
| `tasks_failed_window`, `tasks_window`, `tasks_pending` | toutes les tâches des 30 derniers jours : en échec, total, et encore en attente | |
| `tasks_failed_recent` | tâches en échec que cet utilisateur peut voir, plus récentes que la fenêtre des tâches en échec et non rejetées | |
| `task_failed` | valeur 1 par tâche de ce type (32 au plus) | `task`, `file` |
| `documents`, `documents_inbox` | documents dans Paperless, et dans la boîte de réception | |
| `status_readable` | 0 quand le compte n'a pas la permission de surveillance système | |
| `scrape_errors` | appels qui ont échoué à la dernière sonde | |

Paperless garde les tâches de chaque propriétaire privées : l'utilisateur
dédié voit les tâches planifiées (contrôle de cohérence, index,
classificateur, mail) par leur nom, tandis que les imports d'autres
utilisateurs sont comptés dans `tasks_failed_window`, qui couvre toutes les
tâches.

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui
s'appliquent : file d'attente de tâches Paperless arrêtée, erreur d'index ou
de contrôle de cohérence Paperless, migrations Paperless en attente, tâche
Paperless en échec, stockage Paperless presque plein, mise à jour Paperless
disponible, plus Appareil inaccessible.

## La page de l'appareil

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'appelle jamais Paperless. Il dit d'abord ce qui ne va pas (un composant
en erreur, des tâches en échec, des migrations en attente), puis les mises à
jour, puis les documents, la boîte de réception, les tâches en attente et en
échec et l'espace disque, puis chaque composant et les tâches en échec par
leur nom.

## Créer un utilisateur en lecture seule dans Paperless-ngx

1. Créez un utilisateur nommé comme suit, avec un mot de passe long et aléatoire, sans statut superutilisateur ni staff, et seulement trois permissions de lecture : tâches, surveillance système et statistiques globales. Avec l'API et votre propre compte, les deux commandes ci-dessous le créent et affichent son jeton.

    ```
    dumbmonit
    curl -u YOUR_USER -H 'Content-Type: application/json' -X POST https://paperless.example.com/api/users/ -d '{"username":"dumbmonit","password":"CHANGE-ME","user_permissions":["view_paperlesstask","view_system_monitoring","view_global_statistics"]}'
    curl -X POST https://paperless.example.com/api/token/ -d 'username=dumbmonit&password=CHANGE-ME'
    ```

2. Dans l'interface web à la place : Users & Groups → Add user, puis dans ses permissions cochez View seulement sur les lignes PaperlessTask, SystemMonitoring et GlobalStatistics. Connectez-vous une fois comme cet utilisateur, ouvrez My Profile et générez son jeton d'API.

3. Cet utilisateur ne lit aucun document du tout : les statistiques globales donnent les compteurs, la surveillance système donne la santé de la base de données, de Redis, de Celery et de l'index, et les tâches montrent celles en échec. Sur Paperless-ngx 2.x, où les deux dernières permissions n'existent pas, la santé et les compteurs sont ignorés et le reste est lu.

4. Dans DumbMonit, saisissez l'adresse, par exemple « paperless.lan » (port 8000) ou « https://paperless.example.com » derrière un reverse proxy, et collez le jeton.

!!! warning
    Quand Redis ou Celery s'arrête, Paperless continue de servir ses pages et documents, mais rien de nouveau n'est importé, ni du dossier de consommation ni du mail, jusqu'à ce que quelqu'un regarde. DumbMonit alerte sur les deux.

Remplacez `CHANGE-ME` par un mot de passe long et aléatoire avant de lancer les
commandes, et `YOUR_USER` par un compte autorisé à gérer les utilisateurs
(curl demandera son mot de passe). Demander l'état système peut prendre
plusieurs secondes tant que Redis est en panne : Paperless l'attend avant de
répondre.

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton d'API (recommandé) | Le jeton de l'utilisateur dédié, envoyé comme `Authorization: Token …`. |
| Nom d'utilisateur / mot de passe | L'utilisateur dédié lui-même, envoyé en authentification HTTP basique. |

Adresse : un nom d'hôte (`paperless.lan`), `host:port`, ou une URL complète
avec un préfixe de chemin ; un `/api` final est accepté. Options : protocole
(HTTP par défaut), port (8000), vérification du certificat, délai de requête
(15 s) et la fenêtre des tâches en échec (24 heures, de 1 à 720).

Validé avec Paperless-ngx 3.2.1 (image officielle, SQLite, Valkey).

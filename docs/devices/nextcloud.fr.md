# Nextcloud

Si Nextcloud sert bien ou est bloqué : mode maintenance laissé actif, une
mise à jour de base de données en attente après une mise à jour, mises à
jour de Nextcloud et des applications, utilisateurs actifs, espace libre
pour le répertoire de données, taille de la base de données, et l'OPcache
de PHP.

DumbMonit lit le `status.php` public et les statistiques de l'application
**serverinfo**, fournie avec Nextcloud, avec un **jeton de surveillance**
(`NC-Token`). Le jeton ouvre ces statistiques et rien d'autre : il ne peut
pas se connecter, lister les utilisateurs, ouvrir un fichier ni changer un
réglage.

La panne à surveiller : **une mise à jour qui ne s'est pas terminée**. Après
une mise à jour, Nextcloud peut rester en mode maintenance, ou attendre
`occ upgrade` ; la page web montre alors un avis que personne ne voit
jusqu'à ce que quelqu'un essaie de synchroniser. Les deux sont lus depuis
`status.php`, qui répond même alors.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_nextcloud_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `maintenance` | 1 tant que le mode maintenance est actif | |
| `needs_db_upgrade` | 1 tant que Nextcloud attend `occ upgrade` | |
| `version_info` | valeur 1 | `version` |
| `update_available` | 1 quand la propre vérification de mise à jour de Nextcloud a trouvé une release plus récente | `available_version` |
| `apps_installed`, `app_updates_available` | applications installées, et combien ont une version plus récente dans la boutique d'applications | |
| `app_update_available` | valeur 1 par application avec une mise à jour (32 au plus) | `app`, `available_version` |
| `active_users` | utilisateurs vus dans les 5 dernières minutes, l'heure et le jour | `window` (`5m`, `1h`, `24h`) |
| `users`, `files`, `shares` | comptes, fichiers et partages sur l'instance | |
| `free_space_bytes` | espace libre où Nextcloud garde ses données | |
| `database_size_bytes` | taille de la base de données | |
| `platform_info` | valeur 1 | `database`, `database_version`, `php_version` |
| `php_memory_limit_bytes` | `memory_limit` de PHP | |
| `opcache_enabled`, `opcache_full` | OPcache activé ; OPcache sans place | |
| `opcache_memory_used_percent`, `opcache_interned_strings_used_percent`, `opcache_hit_rate_percent`, `opcache_oom_restarts` | à quel point OPcache est plein et comme il sert bien | |
| `fpm_active_processes`, `fpm_total_processes`, `fpm_listen_queue`, `fpm_max_children_reached`, `fpm_slow_requests` | pool PHP-FPM, quand Nextcloud tourne derrière PHP-FPM (les images `-fpm`, AIO) | |

Pendant que Nextcloud est en maintenance ou attend une mise à niveau, son
API répond 503 : seuls `maintenance`, `needs_db_upgrade` et `version_info`
sont alors écrits, et l'appareil n'est pas rapporté inaccessible.

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui
s'appliquent : Nextcloud bloqué en maintenance, mise à niveau de base de
données Nextcloud en attente, mise à jour Nextcloud disponible, OPcache
Nextcloud plein, espace libre Nextcloud faible, plus Appareil inaccessible.

## Tâches de fond (cron)

Nextcloud ne dit pas au jeton de surveillance quand les tâches de fond ont
tourné pour la dernière fois : seul un compte avec tous les droits sur
l'instance peut le lire, et DumbMonit n'en demande pas un. Surveillez le
cron avec un [battement de cœur](push.md) à la place : créez-en un dans
DumbMonit, puis appelez son URL à la fin de la ligne cron qui lance les
tâches de fond de Nextcloud, pour qu'il n'appelle que lorsque `cron.php` a
réussi :

```
*/5 * * * * php -f /var/www/html/cron.php && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Avec un intervalle attendu de 5 minutes, le battement de cœur alerte dès
que le cron s'arrête, quelle qu'en soit la raison (un conteneur redémarré
sans son cron, une erreur PHP, les tâches de fond laissées sur AJAX).

## Appareils clients

Nextcloud n'expose aucune liste à l'échelle administrateur des clients de
synchro ou des appareils mobiles via une API. La page personnelle « Devices
& sessions » (Settings → Security) ne liste que le compte actuellement
connecté, portée par une session de navigateur plutôt qu'un jeton d'API, et
le seul équivalent à l'échelle administrateur est la ligne de commande
(`occ user:auth-tokens:list <user>`, un compte à la fois) — pas quelque
chose que le jeton de surveillance peut atteindre, et pas quelque chose que
DumbMonit demande à un administrateur de scripter dans une sonde. L'API
`serverinfo` que DumbMonit lit n'a pas non plus de liste d'appareils : elle
donne des compteurs d'utilisateurs actifs (`active_users`, ci-dessus),
jamais qui ou quoi ils sont.

C'est une limite délibérée, pas une fonctionnalité manquante : donner à
DumbMonit une liste d'appareils signifierait élargir le jeton de
surveillance en un compte d'administration complet, ce que le reste de
cette page évite soigneusement. Si Nextcloud expose un jour cela via OCS,
DumbMonit suivra le même schéma
[appareils clients](../alerting/rules.md) déjà utilisé pour Immich,
Proxmox Backup Server et Veeam.

## La page de l'appareil

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'appelle jamais Nextcloud. Il dit d'abord ce qui ne va pas
(maintenance, mise à niveau en attente, OPcache plein ou désactivé), puis
les mises à jour, puis les utilisateurs actifs, l'espace libre, la taille
de la base de données, les utilisateurs, fichiers et chiffres OPcache, puis
les applications à mettre à jour par nom et la plateforme (base de données,
PHP).

## Donner à Nextcloud un jeton de surveillance

1. L'application serverinfo de Nextcloud, fournie et activée avec Nextcloud, répond à un jeton de surveillance qui ouvre ses statistiques et rien d'autre : aucun fichier, aucun utilisateur, aucun partage. Sur le serveur Nextcloud, réglez un jeton long et aléatoire et affichez-le. Lancez occ comme l'utilisateur du serveur web ; en Docker, mettez docker exec -u www-data et le nom du conteneur devant.

    ```
    php occ config:app:set serverinfo token --value "$(openssl rand -hex 32)"
    php occ config:app:get serverinfo token
    ```

2. Si occ dit que serverinfo est désactivé, activez-le d'abord.

    ```
    php occ app:enable serverinfo
    ```

3. Dans DumbMonit, saisissez l'adresse avec laquelle vous ouvrez Nextcloud, par exemple « cloud.example.com » (HTTPS sur 443) ou « http://10.0.0.5:8080 », et collez le jeton. L'hôte doit être l'un des domaines de confiance de Nextcloud, sinon Nextcloud refuse la requête.

4. DumbMonit ne lit que status.php et les statistiques serverinfo. Il n'ouvre jamais un fichier, ne liste jamais les utilisateurs et ne change jamais un réglage, et le jeton ne peut se connecter nulle part.

!!! warning
    Nextcloud ne dit pas au jeton quand les tâches de fond (cron) ont tourné pour la dernière fois : seul un compte avec tous les droits sur l'instance peut le lire. Pour être informé quand le cron s'arrête, ajoutez un battement de cœur dans DumbMonit et appelez son URL à la fin de la ligne cron, comme la documentation le montre.

Avec Nextcloud AIO, lancez occ dans le conteneur `nextcloud-aio-nextcloud` :
`docker exec -u www-data nextcloud-aio-nextcloud php occ …`. Pourquoi pas
un groupe d'administration délégué : Nextcloud n'ouvre l'API serverinfo
qu'au jeton ou aux membres du groupe `admin` complet, et la délégation ne
change pas ça.

Vérifier le jeton à la main, depuis l'hôte DumbMonit :

```
curl -H 'NC-Token: YOUR-TOKEN' 'https://cloud.example.com/ocs/v2.php/apps/serverinfo/api/v1/info?format=json'
```

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton de surveillance (NC-Token) | Le jeton serverinfo. DumbMonit l'envoie dans l'en-tête `NC-Token`, avec `OCS-APIRequest: true`. |

Adresse : un nom d'hôte (`cloud.example.com`), `host:port`, ou une URL
complète avec un préfixe de chemin (`https://example.com/nextcloud`) ; un
`/index.php` ou `/login` final est accepté. Options : protocole (HTTPS par
défaut), port (443), vérification du certificat et délai de requête (15 s).

Le compteur de mises à jour d'applications demande le cache de la boutique
d'applications de Nextcloud ; sur une instance qui ne peut pas atteindre la
boutique, ce compteur reste à 0.

Validé avec Nextcloud 35.0.1 (image officielle).

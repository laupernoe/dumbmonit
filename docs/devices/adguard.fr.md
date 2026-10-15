# AdGuard Home

Le filtre DNS lui-même : si la protection est active, si son serveur DNS
tourne, ce qu'il a répondu et bloqué, si ses listes de filtres se mettent
encore à jour, si ses serveurs amont répondent, et si une version plus
récente est sortie.

Un filtre DNS cassé n'a pas l'air cassé. Les pages continuent de s'ouvrir,
avec leurs publicités. Les pannes habituelles sont silencieuses : protection
mise en pause « pour une heure » depuis l'application téléphone et jamais
réactivée, listes de filtres qui ont arrêté de se télécharger depuis des
semaines, un serveur DNS-over-HTTPS amont qui ne répond plus si bien que
chaque recherche attend le suivant.

DumbMonit lit l'API REST d'AdGuard Home (`/control`) avec une
authentification HTTP basique. Testé avec AdGuard Home v0.107.79 (et la
vérification de version v0.107.78).

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_adguard_`. Les chiffres de
requêtes et de blocages sont des **sommes sur la fenêtre de statistiques
d'AdGuard Home** (24 heures sauf changement dans Settings → General
settings), pas des compteurs : ils montent et descendent au fil de la
fenêtre, et sont stockés tels quels.

| Métrique | Quoi | Labels |
|---|---|---|
| `running` | 1 quand le serveur DNS tourne | |
| `protection_enabled` | 1 quand la protection est active | |
| `protection_paused_seconds` | temps restant avant qu'une protection en pause ne revienne, 0 sans pause | |
| `version_info` | valeur 1 | `version` |
| `dns_queries` | requêtes dans la fenêtre de statistiques | |
| `blocked_filtering` | requêtes bloquées par les listes de filtres et les règles personnalisées | |
| `replaced_safebrowsing`, `replaced_parental`, `replaced_safesearch` | réponses remplacées par Browsing security, Parental control, Safe search | |
| `blocked_percent` | part de requêtes bloquées (filtres, Browsing security, Parental control) ; absent sans aucune requête | |
| `avg_processing_seconds` | temps moyen de réponse | |
| `stats_window_seconds`, `stats_enabled` | la fenêtre de statistiques et si les statistiques sont conservées (0.107.30 et plus récent) | |
| `upstream_responses`, `upstream_avg_seconds` | réponses et temps de réponse moyen par serveur amont dans la fenêtre, comme Top upstreams du tableau de bord les montre (8 au plus) | `upstream` |
| `upstream_up` | 1 quand le propre test d'AdGuard Home sur ce serveur amont réussit ; 0 quand il échoue ou ne donne aucune réponse en 4 secondes (8 au plus). Le label est le serveur amont tel qu'AdGuard Home l'écrit dans ses statistiques (`https://dns10.quad9.net:443/dns-query`, `192.0.2.1:53`) ; pour un serveur amont resté silencieux, la ligne telle que configurée. | `upstream` |
| `filtering_enabled` | 1 quand le filtrage est actif dans Settings → General settings | |
| `filters`, `filters_enabled` | listes de blocage configurées, et activées | |
| `filter_rules` | règles chargées depuis les listes activées | |
| `filter_rules_count`, `filter_update_age_seconds` | par liste activée (32 au plus) | `filter` |
| `filter_oldest_update_age_seconds` | âge de la plus ancienne mise à jour parmi les listes activées | |
| `filters_never_updated` | listes activées jamais téléchargées | |
| `filter_update_interval_hours` | à quelle fréquence les listes sont mises à jour ; 0 désactive les mises à jour | |
| `update_check_enabled` | 0 quand AdGuard Home tourne avec sa vérification de version désactivée | |
| `update_available` | 1 quand la dernière vérification de version a annoncé une version plus récente ; absent quand la vérification est désactivée | `latest` |
| `scrape_errors` | appels secondaires qui ont échoué à la dernière vérification | |

La vérification de version est désactivée dans l'image Docker officielle et
dans la plupart des paquets de distribution, qui mettent AdGuard Home à
jour eux-mêmes : là, `update_available` n'est jamais écrit et sa règle
reste silencieuse.

Les statistiques de serveur amont ne comptent que les réponses : un serveur
amont qui arrête de répondre n'y laisse aucune trace. C'est pourquoi
DumbMonit demande à AdGuard Home de tester chaque serveur amont à chaque
vérification, le même test que le bouton Test upstreams dans Settings → DNS
settings : une requête DNS d'AdGuard Home vers ce serveur amont. Un serveur
amont mort ferait attendre AdGuard Home deux fois son délai amont (20
secondes par défaut) ; DumbMonit arrête d'attendre après 4 secondes et le
rapporte en panne. Décochez l'option Tester les serveurs amont pour
désactiver cela.

Les [règles intégrées](../alerting/rules.md#adguard-home) qui s'appliquent :
protection AdGuard Home désactivée, DNS AdGuard Home ne tourne pas, serveur
amont AdGuard Home en échec, listes de filtres AdGuard Home obsolètes, mise
à jour AdGuard Home disponible, plus Surveillance inaccessible.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page ne se connecte jamais à AdGuard Home. Il dit si la protection et le
serveur DNS sont actifs (et quand une pause se termine), puis les requêtes,
blocages et temps de réponse sur la fenêtre de statistiques, les serveurs
amont avec leur résultat de test et leur temps de réponse, et les listes de
filtres avec leurs règles et dernière mise à jour.

## Ajouter un utilisateur pour DumbMonit dans AdGuard Home

1. AdGuard Home n'a pas de compte en lecture seule : chaque utilisateur listé dans AdGuardHome.yaml peut changer chaque réglage. Donnez à DumbMonit un utilisateur qui lui est propre, pour qu'il puisse être supprimé sans toucher au vôtre. Faites d'abord un hash bcrypt d'un mot de passe long et aléatoire (htpasswd vient avec le paquet apache2-utils ou httpd-tools) :

    ```
    htpasswd -B -C 10 -n -b dumbmonit 'LONG-RANDOM-PASSWORD'
    ```

2. Arrêtez AdGuard Home, ouvrez AdGuardHome.yaml (à côté du binaire AdGuardHome, ou dans le dossier conf du volume Docker) et ajoutez cette ligne sous "users:", à la même indentation que la ligne "- name:" déjà là. Remplacez HASH par ce que htpasswd a affiché après "dumbmonit:". Redémarrez AdGuard Home.

    ```
    - { name: dumbmonit, password: "HASH" }
    ```

    La section lit alors :

    ```yaml
    users:
      - { name: dumbmonit, password: "$2y$10$…" }
      - name: yourname
        password: $2a$10$…
    ```

3. Vérifiez depuis l'hôte DumbMonit que l'utilisateur est accepté :

    ```
    curl -u dumbmonit http://adguard.lan/control/status
    ```

4. Dans DumbMonit, saisissez l'adresse de l'interface web d'AdGuard Home, par exemple « adguard.lan » (port 80) ou « http://adguard.lan:3000 », puis l'utilisateur dumbmonit et son mot de passe.

5. DumbMonit ne lit que le statut, les statistiques, les listes de filtres et la dernière vérification de version. Il ne lit jamais le journal de requêtes. À chaque vérification, il demande à AdGuard Home de tester chaque serveur DNS amont, comme le fait le bouton Test upstreams ; décochez Tester les serveurs amont ci-dessous pour arrêter cela.

!!! warning
    L'utilisateur dumbmonit peut changer chaque réglage, comme tout utilisateur AdGuard Home : gardez son mot de passe pour DumbMonit seul. Il voyage avec chaque requête, donc sur un réseau non sûr activez HTTPS dans Settings → Encryption settings. Après cinq mots de passe incorrects, AdGuard Home refuse l'adresse pendant 15 minutes.

AdGuard Home doit être arrêté pendant que vous éditez le fichier : il
réécrit AdGuardHome.yaml chaque fois qu'un réglage change, et perdrait une
ligne ajoutée pendant qu'il tourne.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur / mot de passe | L'utilisateur dédié ajouté à AdGuardHome.yaml, envoyé en authentification HTTP basique. |
| Aucune authentification | Seulement pour un AdGuard Home sans aucun utilisateur dans AdGuardHome.yaml. |

Adresse : un nom d'hôte ou une IP (`adguard.lan`), `host:port`, ou une URL
complète avec un préfixe de chemin derrière un reverse proxy
(`https://dns.lan/adguard`) ; un `/control` final est ignoré. Options :
protocole (HTTP par défaut), port (80), vérification du certificat, délai
de requête (5 s) et le test amont (activé).

## Appels

| Appel | Pourquoi |
|---|---|
| `GET /control/status` | en fonctionnement, protection, version : le seul appel dont l'échec fait échouer la vérification |
| `GET /control/stats`, `GET /control/stats/config` | requêtes, blocages, réponses amont, fenêtre de statistiques |
| `GET /control/filtering/status` | listes de filtres |
| `POST /control/version.json` avec `{"recheck_now": false}` | le résultat de la dernière vérification de version, comme l'interface web la lit à chaque chargement de page ; ne force jamais une nouvelle vérification |
| `GET /control/dns_info`, puis `POST /control/test_upstream_dns` une fois par serveur amont | les serveurs amont configurés et les serveurs d'amorçage, puis le test amont ; ne change aucun réglage |

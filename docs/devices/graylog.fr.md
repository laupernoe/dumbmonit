# Graylog

Le serveur de logs et le cluster de recherche derrière lui : si Graylog
traite les messages, combien attendent dans son journal, à quel point ses
buffers sont pleins, ce qui entre et ce qui sort, la santé d'OpenSearch ou
Elasticsearch telle que Graylog la voit, les inputs qui n'ont pas démarré, et
les messages qui n'ont pas pu être indexés.

DumbMonit lit l'API REST de Graylog avec un utilisateur dédié portant le rôle
intégré **Reader**. Il ne cherche jamais de messages, ne démarre ni n'arrête
jamais un input et n'acquitte jamais une notification.

La panne à surveiller : **le cluster de recherche qui ne tient plus le
rythme**. Graylog continue d'accepter les messages et les écrit sur son
journal disque ; l'interface web s'ouvre, les recherches fonctionnent, et
montrent simplement rien de récent. Le journal grandit jusqu'à être plein,
puis les messages sont abandonnés. Le retard du journal, la santé du cluster
de recherche et les échecs d'indexation le disent bien avant.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_graylog_`. Le journal, les
buffers et le débit sont ceux **du nœud auquel DumbMonit parle** : dans un
cluster Graylog, ajoutez chaque nœud comme son propre appareil.

| Métrique | Quoi | Labels |
|---|---|---|
| `processing` | 1 tant que le nœud traite des messages, 0 quand le traitement est en pause | |
| `running`, `lb_alive` | le cycle de vie du nœud est `running` ; il se rapporte vivant aux répartiteurs de charge | |
| `version_info` | valeur 1 | `version` |
| `journal_uncommitted_entries` | messages écrits dans le journal et pas encore traités | |
| `journal_size_bytes`, `journal_size_limit_bytes`, `journal_used_percent` | | |
| `journal_append_per_second`, `journal_read_per_second` | | |
| `buffer_used_percent` | à quel point les buffers d'entrée, de traitement et de sortie sont pleins | `buffer` |
| `messages_in_total`, `messages_out_total` | messages reçus, et écrits vers le cluster de recherche | |
| `indexer_status` | santé du cluster de recherche : 0 vert, 1 jaune, 2 rouge, 3 inaccessible depuis Graylog | |
| `indexer_shards` | shards par état : `active`, `initializing`, `relocating`, `unassigned` | `state` |
| `inputs_running`, `inputs_failed` | inputs sur ce nœud | |
| `input_failed` | valeur 1 par input qui n'a pas démarré | `input` (son titre) |
| `output_failures_total`, `processing_failures_total`, `invalid_timestamps_total` | écritures vers le cluster de recherche en échec, messages dont le traitement ou l'indexation a échoué, messages avec un horodatage invalide | |
| `notifications`, `notifications_urgent`, `notification` | propres notifications de Graylog (permission optionnelle, voir ci-dessous) | `type`, `severity` |
| `cluster_nodes` | nœuds dans le cluster Graylog | |
| `scrape_errors` | appels qui ont échoué à la dernière sonde | |

Le cluster de recherche est lu via Graylog, un appareil OpenSearch ou
Elasticsearch séparé n'est donc pas nécessaire pour savoir s'il est vert ;
quand Graylog ne peut pas l'atteindre du tout, `indexer_status` lit 3.

Les [règles intégrées](../alerting/rules.md#log-and-metrics-servers) qui
s'appliquent : Graylog ne traite pas, cluster de recherche Graylog en panne,
cluster de recherche Graylog jaune, journal Graylog qui se remplit, input
Graylog en échec, échecs d'indexation Graylog, plus Appareil inaccessible.

## La page de l'appareil

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais Graylog. Il dit ce qui ne va pas en une phrase
(traitement, cluster de recherche, retard du journal, inputs en échec par
nom, échecs d'indexation, notifications urgentes), puis les messages entrants
et sortants par seconde, le retard du journal et le nombre de nœuds, puis les
buffers, les shards par état et les propres notifications de Graylog.

## Créer un utilisateur et un jeton en lecture seule dans Graylog

1. Dans Graylog : System → Users and Teams → Create user. Nommez l'utilisateur comme suit, donnez-lui un mot de passe long et aléatoire, et attribuez seulement le rôle Reader. Reader peut lire l'état du nœud, le journal, les buffers, le débit, les inputs et la santé du cluster de recherche, et aucun message du tout tant qu'aucun flux ne lui est partagé.

    ```
    dumbmonit
    ```

2. Ouvrez la page Edit tokens de cet utilisateur, créez un jeton nommé d'après DumbMonit et copiez-le maintenant : Graylog ne le montre qu'une fois.

3. Optionnel : les propres notifications de Graylog (un input qui n'a pas démarré, des seuils de disque, un journal presque plein) demandent une permission supplémentaire, notifications:read, qu'aucun rôle intégré n'accorde seul. Avec un compte autorisé à gérer les rôles, créez un rôle ne portant que cette permission et donnez-le à l'utilisateur dumbmonit. Sans elle, les notifications sont ignorées et rien d'autre ne change.

    ```
    curl -u YOUR_USER -H 'X-Requested-By: cli' -H 'Content-Type: application/json' -X POST https://graylog.lan/api/roles -d '{"name":"DumbMonit Notifications","description":"Read system notifications","permissions":["notifications:read"],"read_only":false}'
    curl -u YOUR_USER -H 'X-Requested-By: cli' -X PUT 'https://graylog.lan/api/roles/DumbMonit%20Notifications/members/dumbmonit'
    ```

4. Dans DumbMonit, saisissez l'adresse du nœud Graylog, par exemple « graylog.lan » (port 9000) ou « https://graylog.lan » derrière un reverse proxy, et collez le jeton. Dans un cluster Graylog, ajoutez chaque nœud : le journal, les buffers et le débit sont par nœud.

5. DumbMonit ne fait que lire. Il ne cherche jamais de messages, ne démarre ni n'arrête jamais un input et n'acquitte jamais une notification.

!!! warning
    Ne réutilisez pas le compte avec lequel vous vous connectez : un jeton qui fuit porte tous les droits de son utilisateur, tandis que le rôle Reader ci-dessus ne peut rien changer. Graylog sert du HTTP simple sauf si vous avez configuré TLS, et le jeton voyage avec chaque requête : sur un réseau non sûr, mettez Graylog derrière HTTPS.

Ce que couvre chaque permission de Reader : `system:read` l'état du nœud et
les nœuds du cluster, `journal:read` le journal, `metrics:read` et
`throughput:read` le débit et les buffers, `inputs:read` les inputs,
`indexercluster:read` la santé du cluster de recherche. Le rôle Reader porte
aussi `messages:read`, qui ne lui laisse lire des messages que depuis des
flux explicitement partagés avec lui : n'en partagez aucun. Les détails
d'échec d'indexation (`indices:failures`) ne sont pas nécessaires : DumbMonit
compte les échecs depuis les propres métriques de Graylog.

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton d'accès (recommandé) | Un jeton de l'utilisateur dédié. Graylog l'attend en authentification HTTP basique avec le jeton comme nom d'utilisateur et le mot `token` comme mot de passe ; DumbMonit le fait pour vous. |
| Nom d'utilisateur / mot de passe | L'utilisateur dédié lui-même. |

Adresse : un nom d'hôte ou une IP (`graylog.lan`), `host:port`, ou une URL
complète (`https://graylog.lan`) ; un `/api` final est accepté. Options :
protocole (HTTP par défaut), port (9000), vérification du certificat et
délai de requête (10 s).

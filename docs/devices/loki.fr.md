# Grafana Loki

Le serveur de logs lui-même : s'il est prêt, quelles lignes il refuse et
pourquoi, si ses chunks atteignent le stockage, et si son journal
d'écriture anticipée a encore du disque.

Loki publie `/ready` et `/metrics` sur son port HTTP ; DumbMonit ne lit que
ces deux-là. Il ne lance jamais de requête LogQL et ne lit jamais une ligne de
log.

La panne à surveiller est discrète. Quand Loki refuse une ligne (plus
ancienne que `reject_old_samples_max_age`, au-delà du débit d'ingestion ou de
la limite de flux du tenant, plus longue que `max_line_size`), l'émetteur
reçoit un 4xx, et Promtail, Alloy ou Fluent Bit abandonnent généralement le
lot. Grafana ne montre rien d'autre qu'un trou, découvert le jour où quelqu'un
cherche le log d'un incident. La seconde est les chunks qui ne peuvent pas
être écrits vers le stockage : Loki les garde en mémoire, et les perd au
prochain redémarrage.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_loki_`. Les totaux sont des
compteurs : graphiques et règles les transforment en débits.

| Métrique | Quoi | Labels |
|---|---|---|
| `ready` | 1 quand `/ready` répond `ready`, 0 pendant le démarrage de Loki ou quand un composant ne peut pas écrire ou lire | |
| `version_info` | valeur 1 | `version` |
| `received_lines_total`, `received_bytes_total` | ce que le distributeur a reçu, tous tenants | |
| `discarded_lines_total` | lignes refusées, toujours présent, même à zéro | |
| `discarded_lines_reason_total` | la même chose par raison : `greater_than_max_sample_age`, `rate_limited`, `stream_limit`, `line_too_long`, `per_stream_rate_limit`… | `reason` |
| `flush_failures_total`, `flush_queue_length` | chunks qui n'ont pas atteint le stockage, chunks en attente de l'atteindre | |
| `memory_streams` | flux gardés en mémoire par l'ingester | |
| `wal_disk_full_failures_total`, `wal_disk_used_percent` | écritures du journal d'écriture anticipée perdues pour un disque plein, et à quel point ce disque est plein | |
| `requests_total`, `request_errors_total` | requêtes HTTP et gRPC, et celles répondues par un 5xx ou une erreur gRPC. Un 4xx est la faute du client et n'est pas compté comme une erreur. | |
| `ring_unhealthy_members` | membres des anneaux de hachage marqués défaillants | |
| `panics_total`, `log_errors_total` | panics récupérées, messages d'erreur journalisés | |
| `retention_last_run_timestamp_seconds` | dernière exécution réussie de la rétention par le compactor, quand la rétention est activée | |
| `memory_resident_bytes` | | |

Loki divisé en composants (simple scalable, microservices) expose ces
familles là où elles appartiennent : lignes refusées sur le distributeur,
flush et WAL sur l'ingester. Ajoutez les composants que vous voulez
surveiller ; une famille qu'un composant n'a pas ne produit simplement aucune
série.

Les [règles intégrées](../alerting/rules.md#log-and-metrics-servers) qui
s'appliquent : Loki non prêt, Loki refuse des lignes de log, Loki ne peut pas
écrire vers le stockage, disque du journal d'écriture anticipée de Loki
plein, erreurs de requête Loki, plus Appareil inaccessible.

## La page de l'appareil

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais Loki. Il dit ce qui ne va pas en une phrase (état de
préparation, lignes refusées dans les dernières 24 heures, chunks qui n'ont
pas pu être écrits, disque WAL plein, membres de l'anneau défaillants, erreurs
serveur), puis les lignes et le volume reçus par seconde, les flux en mémoire
et le disque WAL, puis les lignes refusées dans les dernières 24 heures par
raison.

## Laisser DumbMonit lire la santé de Loki

1. Rien à installer : Loki publie /ready et /metrics sur son port HTTP (3100 par défaut). Vérifiez depuis l'hôte DumbMonit que les deux répondent.

    ```
    curl http://loki.lan:3100/ready
    curl -s http://loki.lan:3100/metrics | grep loki_build_info
    ```

2. Loki n'a pas de connexion propre. S'il se trouve derrière un reverse proxy qui demande un nom d'utilisateur et un mot de passe, choisissez Nom d'utilisateur / mot de passe ci-dessous ; pour un jeton bearer, choisissez Jeton bearer. Aucun tenant n'est nécessaire : /ready et /metrics ne sont pas par tenant.

3. Dans DumbMonit, saisissez l'adresse de Loki, par exemple « loki.lan », ou « https://logs.lan/loki » derrière un reverse proxy. Avec Loki divisé en composants (simple scalable ou microservices), ajoutez au moins le chemin d'écriture, où les lignes refusées et les échecs de flush sont comptés.

4. DumbMonit ne lit que /ready et /metrics. Il ne lance jamais de requête LogQL et ne lit jamais une ligne de log.

!!! warning
    Les lignes refusées sont la panne à surveiller : quand Loki refuse une ligne (trop ancienne, au-delà du débit d'ingestion ou de la limite de flux, ligne trop longue), l'émetteur reçoit une erreur et abandonne généralement le lot. Rien ne s'affiche dans Grafana sauf un trou dans les logs.

## Identifiants

| Identifiant | Champs |
|---|---|
| Aucune authentification | Le défaut : Loki lui-même n'a pas de connexion. |
| Nom d'utilisateur / mot de passe | L'authentification basique du reverse proxy devant Loki. |
| Jeton bearer | Un jeton que votre reverse proxy attend dans `Authorization: Bearer`. |

Adresse : un nom d'hôte ou une IP (`loki.lan`), `host:port`, ou une URL
complète avec un préfixe de chemin (`https://logs.lan/loki`). Options :
protocole (HTTP par défaut), port (3100), vérification du certificat et délai
de requête (10 s).

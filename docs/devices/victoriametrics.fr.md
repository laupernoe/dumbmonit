# VictoriaMetrics et VictoriaLogs

La base de séries temporelles et la base de logs elles-mêmes : si elles
acceptent ce qu'on leur envoie, combien de disque reste avant qu'elles
n'acceptent plus rien, et ce qu'elles refusent.

Deux types de surveillance partagent cette page, parce que les deux produits
partagent leurs fondations : `victoriametrics` pour un VictoriaMetrics
mono-nœud, et `victorialogs` pour un VictoriaLogs mono-nœud. Les deux
publient leur propre santé sur `/health` et `/metrics`, sur le port de leur
API ; DumbMonit ne lit que ces deux-là. Il ne lance jamais de requête contre
vos données et n'écrit jamais.

Ces serveurs tombent rarement en panne franche. Ils tombent *à moitié* : ils
continuent de répondre aux requêtes et les tableaux de bord s'ouvrent
toujours, tandis que chaque nouvel échantillon ou ligne de log est refusé,
parce que le disque a passé la limite de lecture seule ou que la donnée
arrivait avec un horodatage hors rétention. Ce qui leur envoie des données
(vmagent, Prometheus remote write, Vector, Fluent Bit, Promtail) réessaie,
puis abandonne. Cette page sert à voir ça venir.

## Ce qu'il surveille

Les métriques sont préfixées `dumbmonit_victoriametrics_` ou
`dumbmonit_victorialogs_`. Les totaux sont des compteurs : graphiques et
règles les transforment en débits.

| Métrique | Quoi | Labels |
|---|---|---|
| `healthy` | 1 quand `/health` répond OK, 0 quand il répond autre chose | |
| `version_info` | valeur 1 | `version` |
| `rows_ingested_total` | échantillons (VictoriaMetrics) ou lignes de log (VictoriaLogs) acceptés | |
| `bytes_ingested_total` | volume de logs accepté (VictoriaLogs seulement) | |
| `rows_rejected_total` | tout ce qui est refusé, toujours présent, même à zéro | |
| `rows_rejected_reason_total` | la même chose, par raison : `big_timestamp`, `small_timestamp`, `too_long_label_value`, `too_many_labels`, `invalid`, `hourly_series_limit`… (VictoriaMetrics) ; `too_big_timestamp`, `too_small_timestamp`, `too_many_fields`, `too_long_line` (VictoriaLogs) | `reason` |
| `disk_free_bytes`, `disk_total_bytes` | espace libre et total du volume de stockage | `path` |
| `disk_free_limit_bytes` | `-storage.minFreeDiskSpaceBytes` : en dessous, le stockage passe en lecture seule | `path` |
| `disk_headroom_percent` | espace libre restant au-dessus de cette limite, en part du disque | `path` |
| `read_only` | 1 une fois que le stockage est passé en lecture seule | |
| `data_size_bytes` | données sur disque | |
| `slow_inserts_total`, `rows_added_total` | nouveaux échantillons qui ont pris le chemin lent, et tous les échantillons ajoutés (VictoriaMetrics seulement) | |
| `active_series` | séries qui ont reçu un échantillon dans la dernière heure (VictoriaMetrics seulement) | |
| `http_errors_total` | requêtes répondues avec une erreur | |
| `log_errors_total` | messages d'erreur journalisés par le serveur | |
| `uptime_seconds`, `memory_resident_bytes` | | |

**Un échantillon refusé est perdu.** VictoriaMetrics refuse un échantillon
plus ancien que sa rétention ou plus de deux jours dans le futur, une
valeur de label plus longue que `-maxLabelValueLen`, une série au-delà de
`-maxLabelsPerTimeseries`, et chaque échantillon d'une nouvelle série une
fois `-storage.maxHourlySeries` ou `-storage.maxDailySeries` atteint.
VictoriaLogs refuse les lignes hors rétention et les lignes avec trop de
champs. Le client n'obtient généralement aucune erreur pour cela : seul ce
compteur le sait.

**Les insertions lentes** sont des échantillons dont VictoriaMetrics a dû
chercher la série sur disque plutôt que dans son cache. Chaque série toute
neuve en est une, un déploiement cause donc une rafale ; une part qui reste
au-dessus de 5 % signifie que le serveur manque de mémoire pour ses séries
actives. C'est le seuil que donne la documentation de VictoriaMetrics.

Les [règles intégrées](../alerting/rules.md#log-and-metrics-servers) qui
s'appliquent : stockage de métriques ou de logs en lecture seule, stockage
de métriques ou de logs presque en lecture seule, serveur de métriques ou
de logs en mauvaise santé, données de métriques ou de logs refusées,
insertions lentes VictoriaMetrics, plus Surveillance inaccessible.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais le serveur. Il dit, dans cet ordre :

1. **Ce qui ne va pas, en une phrase** : santé, stockage (*Lecture seule*,
   ou combien de disque reste au-dessus de la limite), données refusées
   dans les dernières 24 heures, insertions lentes, erreurs journalisées.
   Chaque ligne porte un mot ainsi qu'une couleur.
2. **Ce que fait le serveur** : échantillons ou lignes ingérés par seconde,
   séries actives ou volume ingéré, données sur disque, uptime.
3. **Ce qui a été refusé dans les dernières 24 heures, par raison**, et
   l'espace libre de chaque chemin de stockage par rapport à sa limite de
   lecture seule.

## Laisser DumbMonit lire la santé de VictoriaMetrics

1. Rien à installer : vérifiez depuis l'hôte DumbMonit que VictoriaMetrics répond sur son port d'API (8428 par défaut).

    ```
    curl http://victoriametrics.lan:8428/health
    ```

2. S'il tourne avec -httpAuth.username et -httpAuth.password, choisissez Nom d'utilisateur / mot de passe ci-dessous ; derrière vmauth ou un proxy qui attend un jeton bearer, choisissez Jeton bearer.
3. Dans DumbMonit, saisissez l'adresse du serveur, par exemple "victoriametrics.lan" ou "http://10.0.0.5:8428".

!!! warning
    L'espace disque libre est ce qu'il faut surveiller : sous -storage.minFreeDiskSpaceBytes (100 Mo par défaut), VictoriaMetrics passe en lecture seule et refuse chaque nouveau échantillon. DumbMonit montre la marge restante et alerte sous 10 %.

Documentation du fournisseur : [VictoriaMetrics monitoring](https://docs.victoriametrics.com/victoriametrics/single-server-victoriametrics/#monitoring).

### Bon à savoir

Si `-metricsAuthKey` est défini, `/metrics` ne répond qu'avec cette clé dans
l'URL, que DumbMonit n'envoie pas : protégez plutôt `/metrics` par
l'authentification basique ci-dessus. Derrière un reverse proxy, saisissez
l'URL complète avec son préfixe de chemin. Cela couvre le serveur single-node ;
les composants d'un cluster VictoriaMetrics exposent d'autres métriques et ne
sont pas couverts. DumbMonit ne lit que `/health` et `/metrics` : il n'exécute
jamais de requête sur vos données et n'écrit jamais rien.

## Laisser DumbMonit lire la santé de VictoriaLogs {#let-dumbmonit-read-victorialogs-own-health}

1. Rien à installer : vérifiez depuis l'hôte DumbMonit que VictoriaLogs répond sur son port d'API (9428 par défaut).

    ```
    curl http://victorialogs.lan:9428/health
    ```

2. S'il tourne avec -httpAuth.username et -httpAuth.password, choisissez Nom d'utilisateur / mot de passe ci-dessous ; derrière vmauth ou un proxy qui attend un jeton bearer, choisissez Jeton bearer.
3. Dans DumbMonit, saisissez l'adresse du serveur, par exemple "victorialogs.lan" ou "http://10.0.0.6:9428".

!!! warning
    L'espace disque libre est ce qu'il faut surveiller : sous -storage.minFreeDiskSpaceBytes (10 Mo par défaut), VictoriaLogs passe en lecture seule et refuse chaque nouvelle ligne de log. DumbMonit montre la marge restante et alerte sous 10 %.

Documentation du fournisseur : [VictoriaLogs monitoring](https://docs.victoriametrics.com/victorialogs/#monitoring).

### Bon à savoir

Si `-metricsAuthKey` est défini, `/metrics` ne répond qu'avec cette clé dans
l'URL, que DumbMonit n'envoie pas : protégez plutôt `/metrics` par
l'authentification basique ci-dessus. Derrière un reverse proxy, saisissez
l'URL complète avec son préfixe de chemin. DumbMonit ne lit que `/health` et
`/metrics` : il n'exécute jamais de requête LogsQL et ne lit jamais une ligne
de log.

## Identifiants

| Identifiant | Champs |
|---|---|
| Aucune authentification | Le défaut : `/health` et `/metrics` sont ouverts sauf si vous les avez protégés. |
| Nom d'utilisateur / mot de passe | Les `-httpAuth.username` et `-httpAuth.password` du serveur, ou du reverse proxy devant lui. |
| Jeton bearer | Un jeton que vmauth ou votre reverse proxy attend dans `Authorization: Bearer`. |

Adresse : un nom d'hôte ou une IP (`victoriametrics.lan`, `10.0.0.5`),
`host:port`, ou une URL complète avec un préfixe de chemin derrière un
reverse proxy (`https://metrics.lan/vm`). Options : protocole (HTTP par
défaut), port (8428 ou 9428), vérification du certificat et délai de
requête (10 s).

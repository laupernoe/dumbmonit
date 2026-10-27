# MinIO

!!! warning "Bêta"

    Cette intégration a été construite à partir de la documentation des
    métriques de MinIO, pas encore sur un serveur en fonctionnement. Elle ne
    fait que lire. Dites-nous ce qui ne marche pas.

Un stockage objet MinIO via ses métriques Prometheus : nœuds et disques passés
hors ligne, capacité brute et utilisable, objets et buckets, l'auto-réparation
qui échoue, les échecs de réplication et les requêtes S3 qui finissent en
erreur serveur.

DumbMonit lit `/minio/v2/metrics/cluster` avec le JWT que fabrique
`mc admin prometheus generate`, envoyé en `Authorization: Bearer`, jamais dans
l'URL. Le jeton n'ouvre que les pages de métriques : pas d'accès S3, pas de
console, aucun autre droit. Si MinIO tourne avec
`MINIO_PROMETHEUS_AUTH_TYPE=public`, les pages n'ont pas besoin de jeton. La
page de la grappe est obligatoire ; `/minio/v2/metrics/node` (durée de
fonctionnement du nœud interrogé) est facultative. Les noms de métriques ont
changé d'une version de MinIO à l'autre (`disk` est devenu `drive`) : les deux
sont lus, et une famille que le serveur ne publie pas ne produit simplement
aucun chiffre. Le port de la console (9001) n'est pas utilisé.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 9000 | TCP | API S3 et métriques (HTTP ou HTTPS, par défaut) |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_minio_`.

| Métrique | Labels | Signification |
|---|---|---|
| `version_info` | `version`, `commit` | Valeur 1. |
| `nodes_online`, `nodes_offline` | | Nœuds de la grappe. |
| `drives_online`, `drives_offline`, `drives` | | Disques de la grappe. |
| `capacity_raw_bytes`, `capacity_raw_free_bytes`, `capacity_raw_used_percent` | | Capacité brute. |
| `capacity_usable_bytes`, `capacity_usable_free_bytes`, `capacity_usable_used_percent` | | Capacité utilisable, après parité. |
| `usage_bytes`, `objects`, `buckets` | | Données stockées, objets, buckets. |
| `heal_objects_failed` | | Objets que l'auto-réparation en cours n'a pas pu réparer. |
| `heal_idle_seconds` | | Temps écoulé depuis la dernière activité de réparation ; non publié avant la fin de la première. |
| `heal_active` | | 1 quand la réparation a été active dans les cinq dernières minutes. |
| `replication_failed_objects` | | Objets dont la réplication a échoué (publié quand le serveur l'expose). |
| `s3_requests_total`, `s3_requests_4xx_total`, `s3_requests_5xx_total` | | Compteurs de requêtes S3. Les 5xx sont des erreurs serveur ; les 4xx des erreurs du client. |
| `uptime_seconds` | | Durée de fonctionnement du nœud qui répond. |

Les [règles intégrées](../alerting/rules.md#minio) qui s'appliquent :

- **MinIO node offline** : pendant cinq minutes (Avertissement).
- **MinIO drive offline** : pendant cinq minutes (Avis).
- **MinIO capacity high** : capacité utilisable au-dessus de 85 % pendant 30
  minutes (Avis).
- **MinIO heal failures** : pendant 30 minutes (Avis).
- **MinIO replication failures** : pendant 30 minutes (Avis).
- **MinIO S3 server errors** : plus de 5 % des requêtes en 5xx sur dix minutes
  (Avis).

## La page de la surveillance

Le panneau Object store dit en une phrase ce qui ne va pas et montre les nœuds
et disques en ligne, la capacité utilisée et le nombre d'objets.

## Générer un jeton Prometheus pour DumbMonit

1. With the mc client configured for your MinIO (alias myminio), generate the token of the cluster metrics.

    ```
    mc admin prometheus generate myminio cluster
    ```

2. Copy the value after bearer_token in the output: that is the token.

3. In DumbMonit, enter the MinIO address (for example minio.lan), paste the token as Prometheus token and choose https if MinIO serves TLS.

(Les étapes ci-dessus reprennent le texte anglais affiché dans l'application.)

### Droits requis

- **Prometheus token** — N'ouvre que les pages de métriques, aucun accès S3 ni console

!!! warning
    Le jeton n'ouvre que les pages de métriques, mais quiconque le détient peut les lire. Si MinIO tourne avec MINIO_PROMETHEUS_AUTH_TYPE=public, les métriques n'ont pas besoin de jeton : choisissez Aucune authentification.

### Notes

- Les échecs de réplication et les compteurs 5xx viennent de familles que les
  versions récentes de MinIO publient ; une version plus ancienne peut ne pas
  les avoir.

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton Prometheus | Le `bearer_token` affiché par `mc admin prometheus generate`. |
| Aucune authentification | Pour un serveur avec `MINIO_PROMETHEUS_AUTH_TYPE=public`. |

Adresse : le nom d'hôte ou l'IP de n'importe quel nœud, `hôte:port`, ou une
URL. Options :

- **Protocole** (http) : `http` ou `https`.
- **Port** (9000) : le port de l'API S3, pas la console (9001).
- **Accepter un certificat invérifiable** : pour un serveur avec TLS et un
  certificat auto-signé.
- **Délai par requête** (15 s).

## Dépannage

« MinIO accepted the credentials but refused … (403) » : le jeton est
incorrect. Générez-en un nouveau.

« does not look like MinIO cluster metrics » : l'adresse ou le port n'est pas
l'API S3 d'un serveur MinIO (9000, pas la console).

Un nœud d'une grappe distribuée qui ne répond pas est signalé par les autres :
pointez DumbMonit sur un nœud qui reste en ligne, et ajoutez un ping ou un
moniteur de port sur chaque nœud pour voir lequel.

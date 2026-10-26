# Exporter Prometheus

N'importe quel exporter qui sert le format texte de Prometheus : node_exporter,
windows_exporter, cAdvisor, blackbox_exporter, smartctl_exporter, un exporter de
base de données, ou le vôtre. DumbMonit lit sa page `/metrics` à chaque contrôle
et enregistre chaque série : elle apparaît dans l'explorateur et les graphes, et
peut servir dans des règles de seuil.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'exporter :

| Port | Protocole | Usage |
|---|---|---|
| 9100 | TCP | Page de métriques (HTTP, défaut de node_exporter) |

Les autres exporters ont leur port ; la liste **Exporter** remplit le port
habituel : windows_exporter 9182, cAdvisor 8080, blackbox_exporter 9115,
snmp_exporter 9116, smartctl_exporter 9633, mysqld_exporter 9104,
postgres_exporter 9187, redis_exporter 9121, exporter nginx 9113, Prometheus 9090.

## Mise en place

1. Vérifiez que l'exporter répond depuis la machine DumbMonit.
   ```
   curl http://server.lan:9100/metrics
   ```
2. Dans DumbMonit, ajoutez un moniteur **Prometheus exporter**, saisissez l'hôte
   et choisissez l'exporter (ou laissez `custom` et donnez le port).
3. Facultatif : limitez ce qui est enregistré avec **Keep only** ou **Ignore**.
4. Facultatif : si l'exporter est sur un autre site, choisissez un agent relais
   pour le lire de là-bas.

Vous pouvez aussi coller l'URL entière comme adresse, par exemple
`http://server.lan:9100/metrics` ou, pour blackbox_exporter,
`http://bb.lan:9115/probe?target=example.org&module=http_2xx`.

## Ce qui est enregistré

Chaque série de la page est enregistrée sous `dumbmonit_<nom>` avec ses propres
étiquettes, plus `target` (le moniteur) et `host` (son nom). Une étiquette
`target` ou `host` venue de l'exporter est conservée sous `exported_target` ou
`exported_host`. Les séries déclarées `counter` (et les `_bucket`, `_count`,
`_sum` des histogrammes et résumés) sont des compteurs : graphes et règles en
tirent un taux avec `rate()`. Le reste est une jauge. Les valeurs `NaN` et
infinies ne sont pas enregistrées.

Deux séries disent comment la lecture s'est passée :

| Métrique | Sens |
|---|---|
| `scrape_series` | Séries enregistrées par la dernière lecture. |
| `scrape_series_dropped` | Séries écartées car la limite était atteinte. |

## Options

| Option | Défaut | Usage |
|---|---|---|
| Exporter | `custom` | Préréglage du port. |
| Protocole, Port | `http`, port du préréglage | Où l'exporter écoute. |
| Chemin | `/metrics` | Page lue, avec sa requête le cas échéant. |
| Keep only | vide | Expression régulière : seules les métriques dont le nom la satisfait sont enregistrées. |
| Ignore | vide | Expression régulière : les métriques dont le nom la satisfait ne sont pas enregistrées. |
| Limite de séries | 5000 | Au plus ce nombre de séries par lecture (1 à 50000) ; le surplus est compté. |
| Accepter un certificat non vérifiable | non | Pour un exporter HTTPS à certificat privé. |

Les expressions suivent la syntaxe du crate Rust `regex`, non ancrées : écrivez
`^node_` pour un préfixe. Elles portent sur le nom de la métrique, `Keep only`
d'abord.

## Authentification

Aucune, nom d'utilisateur et mot de passe (HTTP basic) ou jeton bearer. Les
secrets sont chiffrés et jamais réaffichés.

## Pannes

Un exporter qui ne répond pas (connexion refusée, délai dépassé, 5xx) met le
moniteur en panne. Un refus d'identifiants (401, 403), une page qui n'est pas du
Prometheus ou un mauvais chemin relèvent de la configuration : c'est affiché sur
le moniteur, sans notifier personne.

## Alertes

Aucune règle n'est livrée, les métriques dépendant de l'exporter. Créez une règle
de seuil sur n'importe quelle série, par exemple
`100 - avg by (target) (rate(dumbmonit_node_cpu_seconds_total{mode="idle"}[5m])) * 100 > 90`
(voir [Règles d'alerte](../alerting/rules.md#creating-a-threshold-rule)).

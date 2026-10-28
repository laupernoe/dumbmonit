# CrowdSec

Le moteur de sécurité : décisions actives et leur provenance, alertes,
bouncers qui ont arrêté de récupérer, lignes de log lues et celles
qu'aucun parseur n'a reconnues, et si la Local API répond.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 6060 | TCP | Métriques Prometheus (par défaut) |
| 8080 | TCP | Seulement avec l'option de vérification de l'API locale (activée par défaut) |

DumbMonit lit les métriques Prometheus de CrowdSec (port 6060) et vérifie
`/health` de la Local API (port 8080). Avec une clé de bouncer optionnelle, il
demande aussi à la Local API les décisions sur 192.0.2.1, une adresse de
documentation : la réponse est toujours vide, et prouve que l'authentification
et la base de données répondent. Il ne crée ni ne supprime jamais une décision
et ne télécharge jamais la liste de bannissement.

La panne à surveiller est un bouncer qui a arrêté de récupérer. Le pare-feu ou
le reverse proxy continue de tourner avec la liste qu'il avait, de nouveaux
attaquants passent, et rien ne se plaint. CrowdSec ne publie pas la dernière
fois qu'un bouncer a récupéré ; DumbMonit le lit depuis le compteur de
requêtes du bouncer, qui arrête de bouger. La seconde est une acquisition qui
a cassé (un fichier de log renommé, un conteneur recréé) : CrowdSec ne lit
rien, ne voit aucune attaque, et reste vert.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_crowdsec_`. Les totaux sont
des compteurs : graphiques et règles les transforment en débits.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `lapi_up` | 1 quand la Local API répond à `/health` | |
| `lapi_decisions_ok` | avec une clé de bouncer : 1 quand la Local API a répondu à la requête de décision | |
| `decisions` | décisions actives, listes de blocage communautaires incluses | |
| `decisions_by_origin` | | `origin` : `crowdsec`, `cscli`, `CAPI`, `lists`… |
| `alerts` | alertes gardées par la Local API, celles communautaires exclues | |
| `bouncers_seen` | bouncers qui ont appelé la Local API depuis le démarrage de CrowdSec | |
| `bouncer_requests_total` | requêtes de chaque bouncer ; quand il arrête de bouger, le bouncer a arrêté de récupérer | `bouncer` |
| `machine_requests_total` | requêtes de chaque agent (machine) | `machine` |
| `lapi_requests_total` | toutes les requêtes à la Local API | |
| `scenario_overflows_total` | scénarios déclenchés par l'agent local : chacun est une alerte | |
| `lines_read_total`, `lines_parsed_total`, `lines_unparsed_total` | lignes de log lues par l'agent, et celles analysées ou non | |
| `source_lines_total`, `source_lines_unparsed_total` | la même chose par source de log | `source` |
| `buckets` | buckets de scénario actuellement ouverts | |
| `alerts_recent` | avec un identifiant de machine : alertes locales des dernières 24 heures lues dans la Local API | |
| `alerts_by_country` | avec un identifiant de machine : ces alertes par pays d'origine (`source.cn`) | `country` |

Les familles de la Local API apparaissent là où la Local API tourne, les
familles de log là où un agent lit des logs ; l'image officielle et une
installation standard font tourner les deux.

Les [règles intégrées](../alerting/rules.md#databases-message-broker-and-security-engine)
qui s'appliquent : Local API CrowdSec en panne, bouncer CrowdSec arrêté de
récupérer, CrowdSec ne lit aucun log, plus Surveillance inaccessible.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais CrowdSec. Il dit ce qui ne va pas en une phrase
(Local API, bouncers, lecture de logs et la part de lignes qu'aucun parseur
n'a reconnues), puis les décisions actives, les alertes gardées, les
scénarios déclenchés dans les dernières 24 heures et les lignes lues par
seconde, puis le temps depuis la dernière récupération de chaque bouncer, les
décisions par origine et la part non analysée de chaque source de log.

Avec un identifiant de machine, une carte des menaces suit : les alertes des
dernières 24 heures, colorées par pays. Les mêmes comptes alimentent la carte
des menaces de la page Sécurité, à côté des blocages des pare-feu et des
menaces de Cloudflare.

## Laisser DumbMonit lire les métriques de CrowdSec

1. Rendez les métriques joignables : réglez listen_addr dans la section prometheus de /etc/crowdsec/config.yaml sur une adresse que DumbMonit peut atteindre, puis redémarrez CrowdSec.

    ```
    sudo sed -i 's/listen_addr: 127.0.0.1/listen_addr: 0.0.0.0/' /etc/crowdsec/config.yaml
    sudo systemctl restart crowdsec
    ```

2. Vérifiez depuis l'hôte DumbMonit que les métriques répondent sur le port 6060, et filtrez ce port pour que seul l'hôte DumbMonit l'atteigne.

    ```
    curl -s http://crowdsec.lan:6060/metrics | grep cs_info
    ```

3. DumbMonit vérifie aussi l'API locale sur le port 8080 du même hôte : décochez Vérifier l'API locale pour un hôte qui ne fait tourner qu'un agent.
4. Facultatif : créez une clé de bouncer et collez-la comme clé d'API du bouncer ci-dessous.

    ```
    sudo cscli bouncers add dumbmonit
    ```

5. Ou, pour la carte des menaces de la page Sécurité : créez un identifiant de machine et choisissez Identifiant de machine ci-dessous. Seul le pays de chaque alerte est gardé.

    ```
    sudo cscli machines add dumbmonit --password 'a-long-password'
    ```

6. Dans DumbMonit, saisissez l'hôte CrowdSec, par exemple "crowdsec.lan".

### Droits requis

- **Clé d'API de bouncer (facultatif)** — Lecture seule : elle ne lit que les décisions
- **Identifiant de machine (facultatif)** — Lit les alertes ; une machine peut aussi envoyer des alertes, gardez-le pour DumbMonit seul

!!! warning
    Un bouncer qui cesse de tirer continue d'appliquer une liste figée : DumbMonit alerte quand les requêtes d'un bouncer s'arrêtent pendant 30 minutes.

Documentation du fournisseur : [CrowdSec Prometheus metrics](https://docs.crowdsec.net/docs/next/observability/prometheus/).

### Bon à savoir

Les métriques n'exigent aucun identifiant : elles contiennent des compteurs, des
noms de bouncers et de machines et des chemins de fichiers de log, jamais une
adresse IP. L'image Docker officielle écoute déjà sur toutes les interfaces.
`listen_uri` dans la section `api.server` doit être joignable pour la
vérification de l'API locale. Une clé de bouncer ne peut que lire les
décisions : DumbMonit demande les décisions sur 192.0.2.1, une adresse de
documentation, la réponse est donc toujours vide et la liste de bannissement
n'est jamais téléchargée. Quand l'API locale et les agents tournent sur des
hôtes différents, ajoutez chaque hôte : décisions, alertes et bouncers sont
comptés là où tourne l'API locale, les lignes de log là où chaque agent les
lit. DumbMonit ne connaît que les bouncers qui ont tiré au moins une fois
depuis le dernier démarrage de CrowdSec.

Un bouncer supprimé avec `cscli bouncers delete` garde son compteur jusqu'au
redémarrage de CrowdSec, et est rapporté comme arrêté jusque là. Un bouncer
qui n'appelle la Local API que lorsqu'il sert une requête (un bouncer reverse
proxy en mode live) peut rester silencieux pendant une demi-heure une nuit
calme ; passez-le en mode stream, ou augmentez la fenêtre de la règle.

## Identifiants

| Identifiant | Champs |
|---|---|
| Métriques seulement | Rien : lit `/metrics`, et `/health` de la Local API. |
| Clé d'API bouncer | La clé affichée par `cscli bouncers add dumbmonit`, envoyée comme `X-Api-Key` à la Local API. |
| Identifiant de machine | Le nom et le mot de passe donnés à `cscli machines add`. DumbMonit se connecte (`/v1/watchers/login`) et lit les alertes locales des dernières 24 heures (`/v1/alerts`, communautaires exclues), en ne gardant que le pays de chacune : aucune adresse n'est enregistrée. |

Adresse : un nom d'hôte ou une IP (`crowdsec.lan`), `host:port` pour les
métriques, ou une URL complète. Options : protocole (HTTP par défaut), port
des métriques (6060), Vérifier la Local API (activé), port de la Local API
(8080), vérification du certificat et délai de requête (10 s). Les éléments
du Hub (parseurs, scénarios, collections) qui sont corrompus ou obsolètes ne
sont pas visibles via les métriques, seulement avec `cscli hub list` sur
l'hôte.

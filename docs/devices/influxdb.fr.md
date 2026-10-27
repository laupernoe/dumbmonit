# InfluxDB

InfluxDB 2 et 3 : santé et disponibilité, temps de marche, mémoire, descripteurs de fichiers et erreurs du serveur.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 8086 | TCP | API d'InfluxDB 2 (port par défaut) |
| 8181 | TCP | Seulement pour InfluxDB 3, dont le port par défaut est 8181 |

Un seul type d'équipement couvre les deux générations ; la version est reconnue à la réponse de `/health`. InfluxDB 2 répond un objet JSON (`status` à pass ou fail) et n'exige aucun jeton. InfluxDB 3 (Core et Enterprise) répond `OK` et demande un jeton porteur, sauf s'il a été lancé avec `--disable-authz health,ping,metrics`.

Les métriques internes viennent de `/metrics`, lues au mieux : une instance qui les refuse garde les séries de santé et de disponibilité.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_influxdb_`.

| Métrique | Quoi | Étiquettes |
|---|---|---|
| `version_info` | valeur 1 | `version`, `generation` |
| `healthy`, `ready` | 1 si `/health` passe, et si le serveur se dit prêt |  |
| `uptime_seconds` | depuis le démarrage du serveur |  |
| `goroutines`, `memory_resident_bytes` | l'intérieur du processus |  |
| `open_fds`, `open_fds_percent` | descripteurs de fichiers, et leur part de la limite |  |
| `http_requests_total`, `http_requests_5xx_total` | requêtes de l'API, et celles répondues par un 5xx (compteurs) |  |

Les [règles intégrées](../alerting/rules.md#more-databases-and-kafka)
qui s'appliquent : InfluxDB en mauvaise santé, InfluxDB pas prêt, Erreurs serveur InfluxDB, plus Surveillance inaccessible.

## Laisser DumbMonit lire la santé d'InfluxDB

1. InfluxDB 2 : rien à créer, /health, /ready et /metrics sont ouverts. Laissez l'identifiant sur Aucune authentification.

2. InfluxDB 3 demande un jeton sur /health et /metrics par défaut : collez un jeton comme jeton d'API, ou lancez le serveur avec --disable-authz health,ping,metrics.

3. Dans DumbMonit, saisissez l'adresse du serveur, par exemple influx.lan, avec le port 8086 pour InfluxDB 2 ou 8181 pour InfluxDB 3.

4. Choisissez HTTPS si le serveur, ou le reverse proxy devant lui, sert du TLS.

!!! warning
    InfluxDB 3 répond 401 sur /health sans jeton, sauf s'il tourne avec --disable-authz. Le jeton ne sert qu'à lire la santé et les métriques.

### Pourquoi ces choix

InfluxDB 2 laisse ses points de santé et de métriques ouverts. Pour InfluxDB 3, n'importe quel jeton valide suffit à les lire ; DumbMonit n'interroge ni n'écrit jamais de données.

## Identifiants

| Identifiant | Champs |
|---|---|
| Aucune authentification | InfluxDB 2, ou InfluxDB 3 lancé avec `--disable-authz`. |
| Jeton d'API | Un jeton, envoyé en jeton porteur. Exigé par InfluxDB 3 sur `/health`. |

Adresse : un nom d'hôte ou une IP (`influx.lan`), `hôte:port`, ou une URL complète. Options : protocole (HTTP par défaut), port (8086, ou 8181 pour InfluxDB 3), vérification du certificat, délai (10 s).

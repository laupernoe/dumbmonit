# Journaux du serveur

DumbMonit écrit ses propres journaux sur disque, permet à un administrateur de
télécharger les derniers jours, et peut les envoyer à un serveur de logs au fil de
l'eau. Tout se trouve dans **Réglages → Système → Journaux du serveur**
(administrateurs seulement).

## Où sont les fichiers

Les journaux sont dans le volume de données, sous `/data/logs/` :

```
/data/logs/dumbmonit-2026-10-20.log      aujourd'hui, en clair
/data/logs/dumbmonit-2026-10-19.log.gz   jours passés, compressés
```

Chaque ligne est un objet JSON (JSON lines) :

```json
{"ts":"2026-10-20T12:34:56.123Z","level":"INFO","target":"dumbmonit_server::scheduler","msg":"cycle done","fields":{"targets":42},"kind":"app"}
```

| Champ | Sens |
| --- | --- |
| `ts` | heure UTC, RFC 3339 |
| `level` | `ERROR`, `WARN`, `INFO` ou `DEBUG` |
| `target` | le module qui a écrit la ligne |
| `msg` | le message |
| `fields` | valeurs structurées attachées au message |
| `kind` | `app` pour l'application, `audit` pour le journal d'audit (connexions, jetons, utilisateurs…) |

Le filtre global `DUMBMONIT_LOG` (par exemple `info` ou `dumbmonit_server=debug`)
décide de ce qui est journalisé. Tout ce qui suit ne voit que ce qui passe ce filtre.

Aucun secret n'est écrit dans les journaux : mots de passe, jetons, clés d'API et
valeurs similaires sont masqués avant l'écriture, sur disque comme vers chaque
destination.

## Rotation, rétention et plafond

- Un nouveau fichier démarre chaque jour (UTC). Le fichier du jour reste en clair ;
  les jours passés sont compressés en gzip.
- **Rétention** : le nombre de jours conservés, 14 par défaut (de 1 à 365). Les
  fichiers plus anciens sont supprimés.
- **Plafond de taille** : la taille totale du dossier, 1024 Mo par défaut (de 16 à
  102400). Quand il est atteint, les jours les plus anciens partent d'abord.

Les deux se règlent dans les Réglages. Si l'écriture disque ne suit pas, ou si le
plafond est atteint, des enregistrements ne sont pas écrits ; la section en donne le
nombre.

## Export

Choisissez les 1, 7, 14 ou 30 derniers jours, ou un nombre de jours personnalisé (jamais
plus que la rétention). La taille estimée du téléchargement s'affiche avant le bouton
**Télécharger les journaux**. L'archive est un `.tar.gz` qui contient les fichiers
quotidiens de la période et `audit.jsonl`, le journal d'audit filtré sur la même période.

Le même téléchargement est disponible pour des scripts avec une session ouverte :
`GET /api/logs/export?days=7`.

## Destinations

Une destination envoie les journaux à un serveur de logs. Vous pouvez en ajouter
plusieurs. Réglages communs :

- **Nom** et **Activée**.
- **Niveau minimal** : `error`, `warn`, `info` (par défaut) ou `debug`. Les
  enregistrements en dessous ne sont pas envoyés. Le filtre global `DUMBMONIT_LOG`
  s'applique d'abord : une destination ne reçoit jamais plus que ce qu'il laisse passer.
- Chaque destination a sa propre file en mémoire de 10 000 enregistrements. Les envois
  se font par lots de 500 au maximum, ou chaque seconde.

**La perte est acceptée.** Si un lot ne part pas (serveur arrêté, refus, délai dépassé),
il est abandonné et compté, pas rejoué, et une file pleine écarte les plus anciens
enregistrements. Un serveur de logs lent ou injoignable ne ralentit donc jamais
DumbMonit. La liste montre, pour chaque destination, les enregistrements envoyés, les
enregistrements perdus et la dernière erreur. La copie sur disque n'est jamais touchée.

**Envoyer un test** envoie un enregistrement et indique si le serveur l'a accepté.

Les secrets (mots de passe, jetons, clés d'API) sont stockés chiffrés et jamais
réaffichés. En modification, laissez-les vides pour conserver ceux qui sont enregistrés.
Si vous changez l'adresse, saisissez-les à nouveau.

### Graylog (GELF) { #graylog-gelf }

Créez une entrée GELF dans Graylog, puis :

| Champ | Sens |
| --- | --- |
| `transport` | `udp` (par défaut), `tcp` ou `http` |
| `address` | `hôte:port` pour UDP et TCP (port 12201 par défaut) ; une URL pour HTTP, par exemple `http://graylog:12201/gelf` |
| `tls` | TLS sur TCP |
| `insecure_tls` | accepter un certificat qui ne se vérifie pas |
| `username`, `password` (secret) | authentification basique HTTP |

UDP découpe les messages de plus de 8192 octets en morceaux GELF. TCP envoie un
document JSON par enregistrement, terminé par un octet nul.

Exemple : `transport` = `udp`, `address` = `graylog:12201`.

### VictoriaLogs { #victorialogs }

| Champ | Sens |
| --- | --- |
| `url` | URL de base, par exemple `http://victorialogs:9428` |
| `stream_fields` | champs qui identifient un flux, `service,host` par défaut |
| `username`, `password` (secret) | authentification basique |
| `token` (secret) | jeton Bearer, à la place d'un mot de passe |

Les enregistrements sont envoyés à
`/insert/jsonline?_msg_field=_msg&_time_field=_time&_stream_fields=service,host`.

Exemple : `url` = `http://victorialogs:9428`.

### Grafana Loki { #grafana-loki }

| Champ | Sens |
| --- | --- |
| `url` | URL de base, par exemple `http://loki:3100` |
| `tenant` | facultatif, envoyé dans `X-Scope-OrgID` |
| `username`, `password` (secret) | authentification basique (Grafana Cloud) |
| `token` (secret) | jeton Bearer, à la place d'un mot de passe |

Les enregistrements sont envoyés à `/loki/api/v1/push` avec les étiquettes de flux
`service_name="dumbmonit"`, `host` et `level`.

Exemple : `url` = `http://loki:3100`. Dans Grafana, requête `{service_name="dumbmonit"}`.

### Elasticsearch / OpenSearch { #elasticsearch-opensearch }

| Champ | Sens |
| --- | --- |
| `url` | par exemple `https://es:9200` |
| `index` | index ou data stream, `dumbmonit-logs` par défaut |
| `insecure_tls` | accepter un certificat qui ne se vérifie pas |
| `username`, `password` (secret) | authentification basique |
| `api_key` (secret) | envoyée dans `Authorization: ApiKey …`, à la place d'un mot de passe |

Les enregistrements partent vers `/_bulk` en NDJSON avec l'action `create`, ce qui
fonctionne aussi avec un data stream.

Exemple : `url` = `https://es:9200`, `index` = `dumbmonit-logs`.

### Syslog { #syslog }

Les messages Syslog suivent la RFC 5424.

| Champ | Sens |
| --- | --- |
| `transport` | `udp` (par défaut), `tcp` ou `tls` |
| `address` | `hôte:port` ; 514 par défaut en UDP et TCP, 6514 en TLS |
| `framing` | TCP et TLS seulement : `octet-counting` (par défaut, RFC 6587 et 5425) ou `newline` |
| `facility` | `local0` (par défaut) à `local7`, `user` ou `daemon` |
| `app_name` | `dumbmonit` par défaut |
| `insecure_tls` | accepter un certificat qui ne se vérifie pas |
| `ca_cert` | autorité de certification PEM facultative à faire approuver en TLS |

Exemple : `transport` = `tls`, `address` = `syslog.example.org:6514`.

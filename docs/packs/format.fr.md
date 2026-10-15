# Référence du format de pack

Un pack est un seul fichier YAML, `pack.yaml`, en version de schéma 1. Les
clés inconnues sont des erreurs, pas des réglages ignorés : une faute de
frappe comme `treshold` est rapportée.

```text
my-pack/
  pack.yaml          le pack lui-même : le seul fichier dont le serveur a besoin
  fixtures/          une réponse capturée par source, nommée d'après son id
    status.json
  expected.prom      ce que l'extraction doit produire depuis les fixtures
```

## En-tête

| Clé | Requis | Signification |
|---|---|---|
| `schema` | oui | Toujours `1`. |
| `id` | oui | Lettres minuscules, chiffres et tirets simples, commençant par une lettre, jusqu'à 40 caractères : `shelly-plug`. Le type de surveillance devient `pack.<id>`, le préfixe de métrique `dumbmonit_<id avec underscores>_`. |
| `version` | oui | [Version sémantique](https://semver.org) du pack : `1.2.0`. |
| `requires` | non | Versions de DumbMonit avec lesquelles le pack fonctionne, comme une exigence semver : `">=0.2"`. Un serveur hors de la plage refuse le pack. |
| `label` | oui | Nom montré dans la liste des types de surveillance. |
| `summary` | non | Une phrase : ce que le pack surveille. |
| `examples` | non | Produits auxquels il s'applique, montrés comme indices. |
| `address_hint` | non | Placeholder du champ adresse : `192.168.1.50`. |
| `default_port` | non | Port utilisé quand l'adresse n'en a pas. |
| `scheme` | non | `http` (défaut) ou `https` ; la surveillance peut le surcharger. |
| `setup` | non | L'avis montré à côté du formulaire de surveillance : `title`, `steps` (une liste ; ce qui suit un retour à la ligne dans une étape est montré comme une commande copiable), `warning`, `doc_url`. |
| `credentials` | non | Types d'identifiant acceptés, par ordre de préférence : `none`, `api_token`, `username_password`. Défaut : `[none]`. Une entrée peut être `{kind, label, help}` pour reformuler le formulaire. |
| `options` | non | Réglages montrés dans le formulaire de surveillance (ci-dessous). |

## Options

Les options sont stockées sur la surveillance et lues par `{{option.<key>}}`
dans les chemins, en-têtes et corps, et par les seuils de règle.

```yaml
options:
  - key: switch_id          # minuscules, chiffres, underscores
    label: Switch channel
    help: 0 for a plug; 0, 1, 2… for each channel of a relay.
    input: number           # text (défaut), number, boolean ou select
    default: 0
    required: false
    placeholder: ""
    choices: []             # les valeurs d'un select
```

Chaque pack qui sonde obtient aussi cinq options de connexion, qu'il ne
peut pas redéclarer : `scheme`, `port`, `insecure_tls`,
`request_timeout_seconds` (1 à 120, défaut 10) et
`allow_private_targets`.

## Sources

Une source est une requête par sonde, toujours envoyée à la propre
adresse de la surveillance.

```yaml
sources:
  - id: status              # minuscules, chiffres, underscores
    type: http              # http ou prometheus
    path: /rpc/Switch.GetStatus?id={{option.switch_id}}
    method: GET             # GET (défaut) ou POST
    format: json            # http seulement : json (défaut) ou text
    headers:
      Accept: application/json
    body: '{"id": {{option.switch_id}}}'   # POST seulement
    auth: auto              # auto (défaut), none, bearer ou basic
```

- `path` doit commencer par un seul `/`. L'URL de requête est l'adresse
  de la surveillance suivie du chemin ; après substitution elle doit garder le
  même schéma, hôte et port, sinon la requête est refusée. Les valeurs
  d'option sont encodées en pourcentage dans les chemins, une valeur ne
  peut donc pas ajouter un segment ou changer l'hôte.
- `auth: auto` envoie un identifiant `api_token` comme
  `Authorization: Bearer …` et un identifiant `username_password` comme
  authentification basique. `none` n'envoie rien implicitement, un
  en-tête peut donc porter l'identifiant à la place :
  `X-Api-Key: "{{credential.token}}"`. `{{credential.token}}`,
  `{{credential.username}}` et `{{credential.password}}` sont acceptés
  **dans les en-têtes seulement** : jamais dans un chemin ou un corps, qui
  finissent dans les logs.
- Les redirections ne sont suivies que vers le même schéma, hôte et port,
  trois au plus. `401` et `403` sont rapportées comme des erreurs
  d'authentification, les autres statuts non-2xx comme des erreurs de
  protocole.
- Une sonde fait au plus 16 requêtes et lit au plus 4 Mio par réponse.

### Sources Prometheus

```yaml
sources:
  - id: metrics
    type: prometheus
    path: /metrics
    keep: [node_load1, node_memory_MemAvailable_bytes]
    rename:
      node_memory_MemAvailable_bytes: memory_available_bytes
    drop:
      fstype: ^(tmpfs|overlay)$
```

- `keep` liste les familles importées telles quelles, avec tous leurs
  labels. Un nom de famille couvre aussi ses séries d'histogramme et de
  résumé (`_bucket`, `_sum`, `_count`). Les compteurs, histogrammes et
  sommes de résumé restent des compteurs ; tout le reste est une jauge.
- `rename` donne à une famille gardée son nom DumbMonit. Un nom qui n'est
  pas en lettres minuscules, chiffres et underscores doit être renommé.
- `drop` abandonne chaque échantillon dont le label correspond à
  l'expression régulière.
- Les labels que DumbMonit règle lui-même (`target`, `host`, `instance`,
  `job`, `tag_*`…) sont gardés comme `exported_<name>`.

## Métriques

Chaque métrique a un nom *sans* préfixe : `power_watts` dans le pack
`shelly-plug` s'écrit `dumbmonit_shelly_plug_power_watts`. Les noms sont
en lettres minuscules, chiffres et underscores, et uniques dans le pack.

```yaml
metrics:
  - name: power_watts
    help: Active power drawn by the load.
    source: status
    kind: gauge             # gauge (défaut) ou counter
    json: $.apower          # exactement un de json, regex ou prom
    scale: 1                # multiplicateur, p. ex. 0.001 pour ms → s
    map: {on: 1, off: 0}    # valeur texte → nombre
    labels: {}
```

| Extracteur | Source | Valeur | `labels` |
|---|---|---|---|
| `json: <JSONPath>` | `http`, format `json` | Chaque nœud que le chemin correspond ([RFC 9535](https://www.rfc-editor.org/rfc/rfc9535) JSONPath, wildcards et filtres inclus) fait une série. | label → JSONPath évalué sur l'objet qui *contient* la valeur : `$.sensors[*].value` avec des labels `sensor: $.name` étiquette chaque valeur avec son `name` voisin. |
| `regex: <expression>` | `http`, format `text` | Chaque correspondance fait une série ; la valeur est le groupe nommé `value`, sinon le premier groupe. | label → nom d'un groupe de capture. |
| `prom: <nom d'échantillon>` | `prometheus` | Chaque échantillon avec ce nom exact. | label → label de source. Vide : tous les labels de source sont gardés. |

Valeurs : les nombres sont pris tels quels ; `true`/`false` deviennent
1/0 ; le texte est recherché dans `map`, ou analysé comme un nombre
quand il n'y a pas de `map`. Une valeur manquante, `null`, ou hors de
`map` ne produit rien pour cette sonde.

## Discover

`discover` transforme chaque élément d'une liste en un ensemble de
séries partageant des labels : une ligne par disque, capteur ou file.

```yaml
discover:
  - name: disks
    source: list
    rows: $.disks[*]        # JSONPath : une ligne par correspondance
    max_rows: 100           # défaut 100, 1 000 au plus
    labels:                 # JSONPath évalué sur chaque ligne
      disk: $.name
      model: $.info.model
    metrics:
      - name: disk_used_bytes
        json: $.used        # JSONPath évalué sur chaque ligne
      - name: disk_healthy
        json: $.state
        map: {ok: 1, degraded: 0}
```

Les métriques de ligne acceptent `name`, `help`, `kind`, `json`, `scale`
et `map`. Les lignes au-delà de `max_rows` sont ignorées.

## Règles

Les règles sont installées avec le pack, comme des règles d'alerte
ordinaires avec l'uid `pack:<id>:<name>`. Elles vous appartiennent alors :
une mise à jour du pack ajoute les règles qu'il n'a pas encore, et ne
réécrit jamais une que vous avez changée ou supprimée.

```yaml
rules:
  - name: overheating       # minuscules, chiffres, underscores
    title: Shelly plug overheating
    description: The internal temperature stayed above the limit for five minutes.
    expr: dumbmonit_shelly_plug_temperature_celsius
    op: ">"                 # >, >=, < ou <=
    threshold: "{{option.max_temperature}}"   # ou un nombre
    for: 5m                 # 30s, 5m, 1h, 1d
    severity: warning       # info, warning (défaut) ou critical
    unit: °C
```

- `expr` est en [MetricsQL](https://docs.victoriametrics.com/metricsql/)
  et doit utiliser au moins une métrique du pack, par son nom complet.
  Tout autre nom `dumbmonit_*` qu'elle cite doit aussi être produit par
  le pack (`dumbmonit_up` excepté) : un pack ne peut pas alerter sur les
  séries d'un autre type.
- Un `threshold` de `{{option.x}}` installe la valeur par défaut de
  l'option, et la propre valeur de chaque surveillance pour cette option
  devient son seuil pour cette règle (une surcharge par surveillance,
  réécrite chaque fois que la surveillance est enregistrée).

## Profils SNMP

Un pack peut apporter des profils SNMP, écrits exactement comme les
intégrés dans
[`profiles/`](https://github.com/laupernoe/dumbmonit/tree/main/profiles).
Ils rejoignent le catalogue SNMP et sont appliqués par `sysObjectID`
comme les autres ; ils ne créent pas de type de surveillance propre.

```yaml
snmp_profiles:
  - id: acme-ups            # l'id du pack, ou commençant par "<pack id>-"
    name: ACME UPS
    match:
      sysobjectid: [1.3.6.1.4.1.99999]
    include: [system]
    metrics:
      - name: acme_ups_battery_percent   # doit commencer par le préfixe du pack
        oid: 1.3.6.1.4.1.99999.1.1.0
        kind: gauge
```

Le catalogue SNMP est lu au démarrage : redémarrez DumbMonit après avoir
installé ou mis à jour un pack avec des profils SNMP.

## Limites

| Limite | Valeur |
|---|---|
| Taille du pack | 256 Kio |
| Sources (requêtes par sonde, redirections incluses) | 16 |
| Taille de réponse | 4 Mio |
| Séries par sonde | 5 000 (le reste est abandonné et journalisé) |
| Métriques déclarées | 256 |
| Règles | 64 |
| Lignes par `discover` | 1 000 |
| Longueur de valeur de label | 200 octets |

## Fixtures et sortie attendue

`dumbmonit pack test <dir>` lit `fixtures/<id de source>.json`, `.txt` ou
`.prom`, lance la même extraction que le serveur, et compare le résultat
avec `expected.prom` : le format texte Prometheus, trié, sans horodatages
et sans les labels de surveillance (`target`, `host`, `tag_*`) que le serveur
ajoute. `--update` écrit `expected.prom` depuis les fixtures ; vérifiez le
diff avant de le commiter.

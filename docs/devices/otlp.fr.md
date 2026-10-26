# Source OpenTelemetry

Reçoit les métriques poussées en OTLP sur HTTP, depuis un OpenTelemetry Collector
ou directement depuis une application instrumentée. Les jauges, sommes et
histogrammes sont enregistrés en séries, tracés dans l'explorateur et utilisables
dans des règles de seuil. Les journaux envoyés au même point d'entrée sont
enregistrés dans VictoriaLogs quand il est activé.

DumbMonit n'interroge rien ici : les données arrivent.

## Points d'entrée

| Route | Usage |
|---|---|
| `POST /api/otlp/v1/metrics` | Métriques, `application/x-protobuf` ou `application/json`, brutes ou en `Content-Encoding: gzip`. |
| `POST /api/otlp/v1/logs` | Journaux, transmis tels quels à VictoriaLogs (`503` tant qu'il est désactivé). |

OTLP sur gRPC n'est pas pris en charge : l'exporteur HTTP de l'OpenTelemetry
Collector (`otlphttp`) et de tous les SDK fait le même travail sur le port de
l'interface web, sans écouteur supplémentaire.

## Mise en place

1. Dans **Settings, API & assistants**, créez un jeton de portée **write**.
   Limitez-le au réseau de votre collecteur et donnez-lui une date d'expiration.
2. Ajoutez un moniteur **OpenTelemetry source** et notez son numéro : il termine
   l'adresse de sa page, `/devices/<numéro>`.
3. Pointez l'exporteur vers DumbMonit avec le jeton et le numéro.

OpenTelemetry Collector :

```yaml
receivers:
  otlp:
    protocols:
      http:
      grpc:

processors:
  # Compteurs et histogrammes doivent être cumulatifs ; ceci convertit les
  # séries delta. Inutile avec les réglages par défaut de la plupart des SDK.
  deltatocumulative:
  batch:

exporters:
  otlphttp/dumbmonit:
    endpoint: https://monit.example.com/api/otlp
    headers:
      Authorization: Bearer dmt_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
      X-DumbMonit-Target: "12"

service:
  pipelines:
    metrics:
      receivers: [otlp]
      processors: [deltatocumulative, batch]
      exporters: [otlphttp/dumbmonit]
    logs:
      receivers: [otlp]
      processors: [batch]
      exporters: [otlphttp/dumbmonit]
```

Un SDK lit les mêmes réglages dans son environnement :

```
OTEL_EXPORTER_OTLP_ENDPOINT=https://monit.example.com/api/otlp
OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf
OTEL_EXPORTER_OTLP_HEADERS=Authorization=Bearer dmt_xxx,X-DumbMonit-Target=12
```

La cible peut aussi être donnée par `?target=<numéro>` dans l'URL.

## Des points aux séries

Les noms prennent des tirets bas (`http.server.duration` devient
`dumbmonit_http_server_duration`). Les attributs des points deviennent des
étiquettes, ainsi que les attributs de ressource `service.name`,
`service.namespace` et `service.instance.id` (`service_name`, …). Les étiquettes
`target` et `host` sont toujours celles du moniteur ; celles de l'émetteur sont
conservées sous `exported_target` et `exported_host`.

| OTLP | Série |
|---|---|
| Gauge | Jauge. |
| Sum, monotone, cumulative | Compteur, nommé `…_total`. |
| Sum, non monotone | Jauge. |
| Histogram, cumulatif | Compteurs `…_bucket{le}` (cumulés), `…_sum`, `…_count`. |
| Summary | Compteurs `…_sum`, `…_count` et une jauge avec `quantile`. |

Non enregistrés, et comptés comme rejetés dans la réponse (`partialSuccess`) :
la temporalité delta, les histogrammes exponentiels, et les points datés de plus
de 30 jours dans le passé ou d'une heure dans le futur. Les traces ne sont pas
acceptées.

## Limites

* 8 Mio par requête (32 Mio décompressé), 50 000 séries par requête.
* 120 requêtes par minute et par jeton (la limite de tout jeton d'API) ; la
  réponse porte les en-têtes `RateLimit-*` et un `429` dit à l'exporteur de
  réessayer.
* 32 étiquettes au plus par série, 256 caractères par valeur.

## Activité et alertes

Le moniteur publie `dumbmonit_otlp_last_receive_seconds` (âge de la dernière
requête acceptée), `dumbmonit_otlp_points_received_total` et
`dumbmonit_otlp_points_rejected_total`. Ces valeurs sont en mémoire : après un
redémarrage du serveur, rien n'est publié avant la requête suivante. Créez une
règle de seuil sur `dumbmonit_otlp_last_receive_seconds` pour être prévenu quand
un exporteur se tait, et d'autres sur les métriques envoyées, par exemple
`dumbmonit_queue_depth > 1000`.

## Sécurité

La portée `write` ouvre aussi la moitié écriture de l'API REST : gardez le jeton
dans les secrets du collecteur, limitez-le par réseau et donnez-lui une date
d'expiration.

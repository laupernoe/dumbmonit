# Métriques

DumbMonit stocke chaque mesure dans VictoriaMetrics et la relit avec
MetricsQL, via le [proxy de métriques](api.md#metrics) du serveur. Cette
page décrit le nommage, les labels et la rétention. Les listes de
métriques par type sont sur les [pages de surveillance](../devices/index.md).

## Nommage

Chaque métrique est préfixée `dumbmonit_`, pour que DumbMonit puisse
partager une instance VictoriaMetrics avec d'autres outils sans
collision. Les noms suivent `dumbmonit_<quoi>_<unité>` :

| Motif | Exemples |
|---|---|
| `…_bytes`, `…_bytes_total`, `…_bytes_used` | `dumbmonit_memory_bytes_total`, `dumbmonit_filesystem_used_bytes` |
| `…_percent` | `dumbmonit_cpu_usage_percent`, `dumbmonit_pbs_datastore_used_percent` |
| `…_seconds`, `…_age_seconds` | `dumbmonit_probe_duration_seconds`, `dumbmonit_proxmox_backup_last_age_seconds` |
| `…_celsius`, `…_volts`, `…_hertz`, `…_watts` | Lectures d'onduleur et de Synology |
| `…_info` | Séries de présence (valeur 1) dont l'information est dans les labels : `dumbmonit_system_info`, `dumbmonit_pbs_version_info` |
| `…_status`, `…_up`, `…_running` | Énumérations ou booléens : `dumbmonit_if_oper_status`, `dumbmonit_proxmox_guest_running` |

Les familles spécifiques à une source portent leur source comme second
préfixe : `dumbmonit_proxmox_*`, `dumbmonit_pbs_*`, `dumbmonit_synology_*`,
`dumbmonit_probe_*` (services). Les profils SNMP et l'agent utilisent des
noms simples (`dumbmonit_if_octets_in`, `dumbmonit_cpu_usage_percent`).

Les compteurs (octets et paquets d'interface, octets de disque, pages
imprimées) sont stockés **bruts**, comme l'appareil les rapporte :
appliquez `rate()` dans vos propres requêtes pour obtenir un débit. La
page de la surveillance trace chaque série `dumbmonit_*` de la surveillance comme
stockée, un compteur apparaît donc comme une ligne montante. Un
redémarrage se montre comme une remise à zéro de compteur, jamais comme un
pic factice.

## `dumbmonit_up`

Après chaque sonde réussie, le serveur écrit `dumbmonit_up = 1` pour
la surveillance. Rien n'est écrit quand une sonde échoue : la série s'arrête
simplement, et la règle « Surveillance inaccessible » mesure l'ancienneté du
dernier échantillon
(`time() - tlast_over_time(dumbmonit_up[7d])`).

Pour les services (`http`, `tcp`, `dns`, `ping`, `tls`),
`dumbmonit_up = 1` signifie « la surveillance a tourné », et
`dumbmonit_probe_success` dit si le service va bien. Les deux signaux
sont gardés séparés volontairement : un `probe_success` de 0 dit que le
service est en panne, une interruption de `up` dit que la surveillance
est en panne.

## `dumbmonit_security_score`

Toutes les 15 minutes, le serveur calcule le
[score de sécurité](../using/security-score.md) de chaque surveillance dont
le type a des contrôles de sécurité, et écrit `dumbmonit_security_score`
(0–100, absent tant que rien ne peut être noté) et
`dumbmonit_security_checks_failed`, les deux étiquetés `target`.

## Labels

| Label | Réglé par | Signification |
|---|---|---|
| `target` | serveur | L'id de la surveillance, comme dans `/api/targets/{id}`. Chaque règle et chaque graphique sélectionne sur lui. |
| `host` | serveur | Le nom de la surveillance. |
| `tag_<key>` | serveur | Chaque étiquette de la surveillance, y compris les options de type (`tag_insecure_tls`, `tag_port`…). Ne mettez jamais un secret dans une étiquette. |
| `probe` | services | `http`, `tcp`, `dns`, `ping` ou `tls`. |
| labels par série | collecteur | `ifname`, `mountpoint`, `device`, `fstype`, `core`, `service`, `container`, `node`, `vmid`, `name`, `storage`, `datastore`, `group`, `volume`, `disk`, `pool`, `repo`, `changes`, `reason`, `version`, `issuer`… comme listés sur chaque page de surveillance. |

Les labels d'identité gagnent toujours sur les labels de collecteur, et
pour les agents ils sont réglés à la réception : une machine ne peut pas
écrire dans les séries d'une autre machine.

!!! note "Préfixe avant septembre 2026"
    Les séries écrites par EzyMonit sont nommées `ezymonit_…`. Le
    préfixe a changé avec le nom du produit et rien ne lit les anciennes
    séries : elles restent dans VictoriaMetrics jusqu'à ce que la
    rétention les retire, graphiques et règles redémarrent depuis la
    mise à jour, et les règles personnalisées nommant encore
    `ezymonit_…` doivent être modifiées.

## Rétention et stockage

Le VictoriaMetrics embarqué garde 12 mois de données
(`DUMBMONIT_VM_RETENTION`) sous `/data/vm`, dans le volume
`dumbmonit-data`. Le serveur groupe ses écritures et les vide toutes les 5
secondes (`DUMBMONIT_FLUSH_INTERVAL_SECS`). Supprimer une surveillance ne
supprime pas ses séries ; elles vieillissent et expirent.

L'historique d'alerte est gardé 90 jours et les références de séries
disparues 60 jours, les deux dans SQLite (voir la
[référence de configuration](configuration.md)).

## Requêtes

- Via le serveur : `GET /api/metrics/query` et `GET /api/metrics/query_range`,
  avec le cookie de session. Les résultats de plage sont plafonnés à
  2 000 points par série.
- Depuis un Prometheus ou Grafana existant : `GET /federate`,
  `GET /metrics` et la source de données `/prometheus`, avec un jeton
  d'API — [ci-dessous](#scraping-dumbmonit).
- Directement, en développement : la surcouche `docker-compose.dev.yml`
  publie le VictoriaMetrics embarqué sur `http://localhost:8428`, avec sa
  propre interface sur `http://localhost:8428/vmui/`. En production il
  n'écoute que sur le loopback du conteneur ; réglez
  `DUMBMONIT_VM_LISTEN=0.0.0.0:8428` et publiez le port vous-même si vous
  voulez Grafana ou un autre outil sur les mêmes données.

```
# Débit d'interface en bits par seconde
rate(dumbmonit_if_octets_in{target="4"}[5m]) * 8

# Disponibilité de chaque service sur 30 jours
avg by (host) (avg_over_time(dumbmonit_probe_success[30d]))

# Chaque surveillance qui n'a pas rapporté depuis 5 minutes
time() - tlast_over_time(dumbmonit_up[7d]) > 300
```

## Sonder DumbMonit {#scraping-dumbmonit}

Si vous faites déjà tourner un Prometheus ou un Grafana, vous n'avez pas
besoin d'une seconde stack. Trois routes se trouvent hors de `/api`, où
un scraper les cherche :

| Route | Ce qu'elle renvoie |
|---|---|
| `GET /metrics` | La santé de l'instance DumbMonit elle-même : planificateur, sondes, écritures, alertes, notifications, agents, base de données. Quelques dizaines de séries, toujours le même nombre — jamais une par surveillance. |
| `GET /federate` | Les mesures, sélectionnées par `match[]`, exactement comme Prometheus fédère un autre Prometheus : le dernier point de chaque série sélectionnée. |
| `GET /prometheus/api/v1/…` | La moitié lecture de l'API Prometheus, relayée vers le magasin : ce qu'une source de données Grafana appelle. |

Les trois prennent `Authorization: Bearer dmt_…` avec la portée
**`read`** — un jeton créé dans Réglages → *API & assistants*, le même
type que le [serveur MCP](../using/assistant.md) utilise. Aucun cookie
n'est impliqué, aucun en-tête `X-Requested-With` n'est donc nécessaire ;
un cookie de session est aussi accepté, vous pouvez donc ouvrir n'importe
laquelle depuis un navigateur connecté. Un jeton est limité à 120 appels
par minute : largement assez pour un scrape toutes les 15 secondes, et
assez pour un tableau de bord Grafana de quelques dizaines de panneaux —
donnez à un tableau de bord chargé son propre jeton plutôt que d'en
partager un avec un scraper.

### Prometheus

```yaml
# prometheus.yml — la propre santé de DumbMonit, puis les mesures qu'il collecte.
scrape_configs:
  - job_name: dumbmonit
    metrics_path: /metrics
    scheme: http
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    static_configs:
      - targets: ['dumbmonit.lan:8080']

  - job_name: dumbmonit-federate
    metrics_path: /federate
    scheme: http
    honor_labels: true
    scrape_interval: 60s
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    params:
      'match[]':
        - '{__name__=~"dumbmonit_.*"}'
    static_configs:
      - targets: ['dumbmonit.lan:8080']
```

`honor_labels: true` garde les labels `target` et `host` que DumbMonit
règle plutôt que de les écraser avec les propres labels du job de scrape.
Mettez le jeton dans un fichier et utilisez
`authorization: {type: Bearer, credentials_file: /etc/prometheus/dumbmonit.token}`
si vous préférez ne pas l'avoir dans la configuration.

Sans `match[]`, `/federate` sélectionne `{__name__=~"dumbmonit_.*"}` —
chaque série que DumbMonit écrit, et rien appartenant à un autre outil qui
partage le même VictoriaMetrics. Restreignez-le pour garder une
fédération petite : `{__name__=~"dumbmonit_.*",target="4"}` pour une surveillance, `{__name__="dumbmonit_up"}` pour la disponibilité seulement. 10
sélecteurs au plus par requête, et une réponse au-delà de 8 Mio est
refusée avec un message vous disant de restreindre — divisez un grand
parc en plusieurs jobs avec des sélecteurs plus étroits plutôt qu'un seul
qui demande tout. `max_lookback=15m` élargit la fenêtre dans laquelle le
dernier point est cherché, pour des surveillances sondées moins souvent que
toutes les cinq minutes.

```bash
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/metrics
curl -H 'Authorization: Bearer dmt_…' -G http://localhost:8080/federate \
  --data-urlencode 'match[]={__name__=~"dumbmonit_.*",target="4"}'
```

### Grafana

Pointez Grafana vers **DumbMonit**, pas vers le VictoriaMetrics qu'il
fait tourner : le processus enfant écoute sur le loopback du conteneur,
n'a pas d'authentification propre, et expose les routes qui *suppriment*
des séries aussi facilement que celles qui les lisent. DumbMonit relaie
la moitié lecture de l'API Prometheus sous `/prometheus`, derrière le
même jeton, pour que Grafana parle à une source de données Prometheus
normale :

1. **Connections → Data sources → Add → Prometheus**.
2. **URL** : `http://dumbmonit.lan:8080/prometheus` (Grafana ajoute
   `/api/v1/query` lui-même).
3. **HTTP headers** : ajoutez-en un, `Authorization` = `Bearer dmt_…`.
   Grafana le stocke chiffré et ne le montre plus jamais.
4. **Save & test** — Grafana rapporte que la source de données
   fonctionne, et chaque série `dumbmonit_*` est alors disponible en
   PromQL, avec l'autocomplétion sur les noms de métrique et de label.

Seules les routes de lecture sont relayées — `query`, `query_range`,
`series`, `labels`, les valeurs de label, `metadata` et la version que
Grafana demande en testant la connexion. Tout le reste,
`admin/tsdb/delete_series` en premier, répond `404` : ce point d'entrée ne
peut rien changer ni effacer.

Si vous faites déjà tourner un Prometheus, l'autre chemin fonctionne
tout aussi bien : Prometheus fédère DumbMonit avec le job ci-dessus,
Grafana lit Prometheus, et les requêtes sont identiques.

### Métriques d'instance

Tout ce que `GET /metrics` expose. Les valeurs sont lues quand vous
scrapez ; les compteurs `_total` sont monotones et ne se réinitialisent
que quand le serveur redémarre (`dumbmonit_uptime_seconds` vous dit
quand ça s'est produit).

| Métrique | Type | Signification |
|---|---|---|
| `dumbmonit_build_info{version}` | gauge | Toujours 1. Le label `version` est la version du serveur en cours, la même que `/api/health` rapporte. |
| `dumbmonit_uptime_seconds` | gauge | Secondes depuis le démarrage du processus serveur. |
| `dumbmonit_scheduler_cycles_total` | counter | Cycles de planificateur exécutés. La boucle tique une fois par seconde, ça double donc comme signal de vie. |
| `dumbmonit_scheduler_cycle_seconds` | gauge | Durée du dernier cycle. Normalement sous une milliseconde : le cycle ne fait que distribuer, il n'attend pas les sondes. |
| `dumbmonit_scheduler_backlog` | gauge | Surveillances encore dues pour une sonde quand le dernier cycle s'est terminé. Zéro sur une instance en bonne santé ; une valeur non nulle qui persiste signifie que `DUMBMONIT_MAX_CONCURRENT_PROBES` est trop bas pour le nombre de surveillances et leurs intervalles. |
| `dumbmonit_probes_total{kind}` | counter | Sondes lancées, par type de surveillance (`snmp`, `proxmox`, `http`…). Inclut celles qu'un agent relais a lancées pour le compte du serveur. |
| `dumbmonit_probes_failed_total{kind}` | counter | Sondes qui ont échoué, par type — une surveillance inaccessible ou une erreur de configuration. `rate(dumbmonit_probes_failed_total[15m]) / rate(dumbmonit_probes_total[15m])` est le taux d'échec. |
| `dumbmonit_samples_written_total` | counter | Échantillons acceptés par VictoriaMetrics. |
| `dumbmonit_sample_writes_failed_total` | counter | Lots que VictoriaMetrics a refusés ou n'a pas répondu. Le lot reste en tampon et est retenté ; une valeur qui monte avec un `dumbmonit_samples_pending` qui monte signifie que le magasin est en panne ou plein. |
| `dumbmonit_samples_pending` | gauge | Échantillons encore dans le tampon d'écriture après le dernier flush. Normalement 0. |
| `dumbmonit_samples_dropped_total` | counter | Échantillons abandonnés avant même d'atteindre le tampon d'écriture, parce qu'il était plein ou fermé. Jamais retenté — une valeur qui monte pendant une panne signifie une perte de données, pas juste un délai. |
| `dumbmonit_alerting_cycles_total` | counter | Cycles d'alerte exécutés (un toutes les `DUMBMONIT_ALERT_INTERVAL_SECS`, 30 s par défaut). |
| `dumbmonit_alerting_cycle_seconds` | gauge | Durée du dernier cycle d'alerte, temps de requête inclus. S'approcher de l'intervalle signifie que les règles interrogent plus que le magasin ne peut servir. |
| `dumbmonit_alerting_rules_evaluated` | gauge | Règles évaluées dans le dernier cycle. |
| `dumbmonit_alerting_rules_failed` | gauge | Règles dont la requête a échoué dans le dernier cycle. Leurs alertes sont gelées, pas résolues. Tout sauf 0 mérite un coup d'œil aux logs. |
| `dumbmonit_alerts{phase}` | gauge | Alertes que le moteur suit, par phase : `ok`, `pending` (condition vraie, `for` pas écoulé), `firing`, `resolved`. |
| `dumbmonit_alerts_suppressed` | gauge | Alertes cachées parce qu'une surveillance parente est en panne (la phase effective `suppressed`). |
| `dumbmonit_alerts_silenced` | gauge | Alertes couvertes par une fenêtre de maintenance. |
| `dumbmonit_alerts_learning` | gauge | Alertes de référence encore en apprentissage, et donc silencieuses. |
| `dumbmonit_notifications_total{kind}` | counter | Notifications envoyées, par type de canal (`email`, `discord`, `ntfy`…). |
| `dumbmonit_notifications_failed_total{kind}` | counter | Notifications que le canal a refusées, par type. Elles sont remises en file et retentées ; une valeur qui monte régulièrement est un webhook ou un compte SMTP cassé. |
| `dumbmonit_agents` | gauge | Machines qui ont enregistré un agent. |
| `dumbmonit_agents_stale` | gauge | Agents qui n'ont rien envoyé depuis cinq minutes. |
| `dumbmonit_database_up` | gauge | 1 quand SQLite répond — le même contrôle que `/api/health`. |
| `dumbmonit_database_bytes` | gauge | Taille de `dumbmonit.db` et de son journal d'écriture anticipée. N'inclut pas `/data/vm`. |
| `dumbmonit_victoriametrics_up` | gauge | 1 quand VictoriaMetrics répond — le même contrôle que `/api/health`. 0 pendant que l'enfant embarqué redémarre. |
| `dumbmonit_victoriametrics_embedded` | gauge | 1 quand ce serveur fait tourner VictoriaMetrics lui-même, 0 quand `DUMBMONIT_VM_URL` pointe vers un externe. |

Aucune métrique sur cette page ne porte un label par surveillance : le
document a la même taille pour une surveillance et pour mille, et aucun nom
de surveillance ne fuit vers qui peut le scraper.

Deux alertes qui valent la peine d'être ajoutées de votre côté, une fois
DumbMonit scrapé :

```
# La surveillance a arrêté de surveiller
up{job="dumbmonit"} == 0

# Les mesures s'accumulent au lieu d'être stockées
dumbmonit_samples_pending > 0 and increase(dumbmonit_sample_writes_failed_total[10m]) > 0
```

### Ouvrir les routes sans jeton

`DUMBMONIT_METRICS_PUBLIC=true` sert `/metrics`, `/federate` et
`/prometheus/api/v1/…` à quiconque peut atteindre le port, pour les gens
qui filtrent le port eux-mêmes — **quiconque sur ce réseau peut alors
lire chaque mesure de chaque surveillance et l'état de l'instance, ne le
réglez donc que là où autre chose restreint déjà qui atteint le port.**

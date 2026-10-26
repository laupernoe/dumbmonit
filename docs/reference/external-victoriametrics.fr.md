# VictoriaMetrics externe / cluster

Par défaut DumbMonit démarre son propre VictoriaMetrics dans le conteneur. Vous
pouvez lui désigner à la place une instance que vous gérez vous-même.

## Quand c'est utile

* Plus d'environ **5 000 surveillances** (voir [Dimensionnement](sizing.md)).
* **Haute disponibilité**, ou stockage sur un hôte ou un disque à part.
* Vous **faites déjà tourner** VictoriaMetrics et voulez un seul endroit pour
  la rétention, les sauvegardes et la supervision.
* Vous voulez lire les séries de DumbMonit aussi depuis **Grafana** ou d'autres
  outils.

Pour un homelab ou une petite équipe, celui embarqué est plus simple et ne
demande rien.

## Instance simple (single-node)

```yaml
    environment:
      DUMBMONIT_VM_URL: http://victoriametrics:8428
```

DumbMonit écrit sur `<URL>/api/v1/import/prometheus` et lit sur
`<URL>/api/v1/query`, `query_range`, `series` et `label/…`. Rien d'autre n'est
requis côté VictoriaMetrics ; ses séries sont préfixées `dumbmonit_`, il peut
donc être partagé.

Quand `DUMBMONIT_VM_URL` est défini, **le VictoriaMetrics embarqué n'est pas
démarré** et `/data/vm` reste vide. `GET /api/health` indique
`victoria.embedded: false`.

## Ce qui change

* **La rétention se règle sur l'instance externe** (`-retentionPeriod`). Dans
  **Réglages → Système**, le champ de rétention brute passe en lecture seule.
* **La mémoire** (`DUMBMONIT_VM_MEMORY`) et l'adresse d'écoute ne s'appliquent
  plus : dimensionnez l'instance externe vous-même.
* **L'effacement de séries** (suppression d'une surveillance, nettoyage) appelle
  `/api/v1/admin/tsdb/delete_series`. VictoriaMetrics single-node l'autorise par
  défaut ; une option `-deleteAuthKey` sur l'instance fait échouer l'appel de
  DumbMonit, le reste fonctionne et les séries vieillissent simplement.
* **Sauvegardes** : la sauvegarde de DumbMonit ne couvre plus les métriques.
  Sauvegardez l'instance avec `vmbackup` ou les instantanés de votre plateforme.
* Le serveur met les échantillons en tampon et réessaie quand l'instance est
  indisponible, dans les limites de son tampon mémoire ; une longue panne perd
  des données.

## Cluster (vminsert / vmselect)

Un cluster VictoriaMetrics sert l'écriture et la lecture sur des services
différents, sous des chemins de locataire :

| Opération | URL du cluster |
|---|---|
| Écriture | `http://vminsert:8480/insert/0/prometheus/api/v1/import/prometheus` |
| Lecture | `http://vmselect:8481/select/0/prometheus/api/v1/query_range` |

DumbMonit construit chaque URL en `DUMBMONIT_VM_URL` + `/api/v1/…`, et une URL
unique doit servir l'écriture comme la lecture. Il ne peut donc pas parler
directement à `vminsert` et `vmselect`. Placez **vmauth** devant et aiguillez
par chemin :

```yaml
# configuration vmauth (-auth.config)
unauthorized_user:
  url_map:
    - src_paths: ["/api/v1/import/.*"]
      url_prefix: "http://vminsert:8480/insert/0/prometheus/"
    - src_paths: ["/api/v1/admin/tsdb/delete_series"]
      url_prefix: "http://vmselect:8481/delete/0/prometheus/"
    - src_paths: ["/api/v1/(query|query_range|series|labels|label/.*|export)"]
      url_prefix: "http://vmselect:8481/select/0/prometheus/"
```

Puis `DUMBMONIT_VM_URL: http://vmauth:8427`. Gardez vmauth sur un réseau privé,
ou donnez-lui des `users:` avec identifiants ; un secret dans l'URL est stocké
dans l'environnement du conteneur, traitez-le comme tout autre secret. Le
locataire `0` n'est qu'un exemple : prenez l'identifiant de compte voulu.

## Stockage longue durée

Les agrégats de 5 minutes vivent dans un second VictoriaMetrics. Avec
l'installation embarquée, il est démarré pour vous sous `/data/vm-long`. Avec
une instance principale externe, il n'y en a pas tant que vous n'en désignez pas
une :

```yaml
    environment:
      DUMBMONIT_VM_URL: http://victoriametrics:8428
      DUMBMONIT_VM_LONG_URL: http://victoriametrics-long:8428
```

Prenez une **instance séparée** (ou un locataire derrière vmauth, avec le même
aiguillage que ci-dessus) : les agrégats portent les mêmes noms de métrique que
le brut, avec les suffixes `:5m_min`, `:5m_max`, `:5m_sum` et `:5m_count`, et
leur rétention (730 jours par défaut) diffère de celle du brut. Réglez
`-retentionPeriod` de cette instance en conséquence. Sans
`DUMBMONIT_VM_LONG_URL`, une instance principale externe signifie **pas
d'historique long** : **Réglages → Système** le dit, et les graphiques se
limitent à la rétention de l'instance externe.

Définir `DUMBMONIT_VM_LONG_URL` avec l'instance principale embarquée fonctionne
aussi : le stockage longue durée embarqué n'est alors pas démarré.

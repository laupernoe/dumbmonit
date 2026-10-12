# Packs d'intégration

Un pack d'intégration ajoute un type d'appareil à DumbMonit sans nouvelle
release. C'est un seul fichier YAML qui dit où lire (une API HTTP ou une
page Prometheus `/metrics` sur l'appareil), quoi extraire, et quelles règles
d'alerte viennent avec. Une fois installé, le type du pack apparaît dans
**Ajouter un appareil** à côté des types intégrés, et ses appareils sont
sondés, affichés en graphique et soumis aux alertes comme n'importe quel
autre.

Un pack est déclaratif : pas de code, pas de scripts. DumbMonit exécute
chaque requête lui-même, sous des règles que le pack ne peut pas changer.

## Ce qu'un pack peut et ne peut pas faire

| Un pack peut | Un pack ne peut pas |
|---|---|
| Appeler des chemins `GET` ou `POST` sur la propre adresse de l'appareil | Appeler un autre hôte, suivre une redirection vers un autre hôte, ou utiliser une URL absolue |
| Lire du JSON (JSONPath), du texte brut (expressions régulières) ou le format texte Prometheus | Exécuter du code, des commandes shell ou des templates au-delà de `{{option.x}}` et `{{credential.x}}` |
| Envoyer l'identifiant de l'appareil dans un en-tête ou en authentification basique/bearer | Mettre un identifiant dans une URL ou un corps de requête |
| Déclarer des options affichées dans le formulaire de l'appareil (un canal d'interrupteur, un seuil) | Écrire des métriques en dehors de son propre préfixe `dumbmonit_<pack>_` |
| Fournir des règles d'alerte sur ses propres métriques | Changer un type d'appareil intégré ou une règle intégrée |
| Apporter des profils SNMP, au format des intégrés | Lire plus de 4 Mio par réponse, faire plus de 16 requêtes par sonde, ou produire plus de 5 000 séries par sonde |

Les adresses loopback et link-local (`127.0.0.1`, `169.254.x.x`) sont
refusées à moins que l'option **Autoriser les cibles loopback et
link-local** de l'appareil ne soit activée, exactement comme les
vérifications HTTP. Les adresses LAN privées sont toujours autorisées.

## Packs de référence

Le dépôt fournit trois packs sous
[`packs/`](https://github.com/laupernoe/dumbmonit/tree/main/packs), chacun
avec ses fixtures et sa sortie attendue :

| Pack | Lit | Règles |
|---|---|---|
| `node-exporter` | un sous-ensemble sélectionné de Prometheus node_exporter (CPU, mémoire, systèmes de fichiers, disques, réseau) | système de fichiers presque plein, mémoire presque épuisée |
| `shelly-plug` | prises et relais Shelly Gen2+, `/rpc/Switch.GetStatus` | surchauffe |
| `speedtest-tracker` | le dernier résultat de l'API v1 de Speedtest Tracker | internet en dessous des attentes, test de vitesse en échec |

## Installer un pack

Dans l'interface, en tant qu'administrateur : **Réglages → Packs
d'intégration**. Collez le YAML du pack ou téléversez son `pack.yaml`, puis
**Installer le pack**. Un pack qui ne passe pas les vérifications est
refusé avec la liste de ce qu'il faut corriger ; un qui passe est enregistré
avec ses notes, s'il y en a. La même page active et désactive les packs et
les désinstalle. Les types installés apparaissent sous **Packs
communautaires** quand vous ajoutez un appareil, marqués *Pack*.

Via l'API HTTP, il vous faut une session d'administrateur ou un jeton d'API
avec la portée `write` (Réglages → **API & assistants**).

```bash
# Installer, ou mettre à jour vers une version plus récente du même pack
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @packs/shelly-plug/pack.yaml http://localhost:8080/api/packs

# Lister ce qui est installé
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/api/packs
```

La réponse dit ce qui s'est passé (`created`, `updated` ou `unchanged`), les
métriques que le pack produit et combien de règles d'alerte ont été
ajoutées. Un pack qui ne passe pas la validation est refusé avec la liste
complète des problèmes, un par ligne.

| Appel | Fait |
|---|---|
| `GET /api/packs` | packs installés : version, type, métriques, règles, appareils qui l'utilisent |
| `GET /api/packs/{id}` | un pack |
| `POST /api/packs` | installer ou mettre à jour ; corps : le YAML (`application/yaml`), ou `{"yaml": "…"}` en JSON |
| `PUT /api/packs/{id}/disable` | arrêter de sonder les appareils du pack, garder tout le reste |
| `PUT /api/packs/{id}/enable` | les sonder à nouveau |
| `DELETE /api/packs/{id}` | désinstaller, avec ses règles ; refusé (`409`) tant que des appareils l'utilisent encore |

Avec un cookie de session plutôt qu'un jeton, ajoutez
`X-Requested-With: DumbMonit` à chaque `POST`, `PUT` et `DELETE`, comme pour
le reste de l'[API](../reference/api.md).

Installer prend effet immédiatement : le nouveau type est disponible dans le
formulaire d'appareil sans redémarrage. La seule exception est un pack qui
apporte des profils SNMP ; la réponse dit alors `"restart_required": true`,
parce que le catalogue SNMP est lu au démarrage.

### Mises à jour et vos changements

Mettre à jour un pack (même `id`, nouveau contenu) remplace son extraction
et sa description. Ses règles d'alerte ne sont jamais que *ajoutées* : une
règle que vous avez changée, désactivée ou supprimée n'est jamais réécrite
par une mise à jour.

Désactiver un pack retire son type d'appareil et garde tout le reste :
appareils, historique, règles. Ses appareils ne sont plus sondés (chacun
montre une erreur de configuration) et, comme tout appareil qui arrête de
rapporter, déclenchent *Appareil inaccessible* après trois minutes : mettez-
les aussi en pause si vous désactivez le pack pour un moment.

### Sauvegardes

Les packs installés voyagent dans les [sauvegardes](../install/backup.md)
avec leur YAML, et sont restaurés avant les appareils qui les utilisent.

## Vérifier un pack avant de l'installer

Le binaire du serveur valide et teste les packs hors ligne :

```bash
# Vérifications de schéma et de cohérence, sans serveur ni base de données
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack lint /packs/shelly-plug

# Rejoue fixtures/ à travers la vraie extraction et compare avec expected.prom
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /packs/shelly-plug
```

`pack test --update` réécrit `expected.prom` depuis les fixtures (montez le
répertoire en lecture-écriture et lancez avec `--user "$(id -u)"` pour que
le fichier reste à vous).

Suite : [écrire votre premier pack](first-pack.md) en environ dix minutes,
ou lisez la [référence du format](format.md).

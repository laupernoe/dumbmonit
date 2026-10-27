# Fichiers de configuration (GitOps)

Décrivez vos moniteurs, règles d’alerte, canaux de notification, fenêtres de
maintenance et pages de statut en YAML, gardez les fichiers dans Git, et laissez
DumbMonit converger vers eux. Trois façons d’utiliser le même format :

- **l’interface web** : *Réglages → Système → Fichiers de configuration* exporte
  l’instance, montre le plan d’un fichier et l’applique ;
- **`dumbmonit-cli`** : `plan` et `apply` depuis un terminal ou un pipeline CI ;
- **un dossier surveillé** : `DUMBMONIT_CONFIG_DIR` est appliqué au démarrage et
  à chaque changement, idéal pour un dépôt Git monté dans le conteneur.

Un [fournisseur Terraform](https://github.com/laupernoe/dumbmonit/tree/main/integrations/terraform-provider-dumbmonit)
gère les mêmes objets un par un par l’API REST.

## Le fichier

```yaml
version: 1

channels:
  - name: Téléphone
    kind: ntfy
    enabled: true
    settings: { server: "https://ntfy.sh", topic: alertes-homelab }
    secrets: { token: "${NTFY_TOKEN}" }

targets:
  - kind: http
    address: https://example.org
    name: Site web
    interval_secs: 60
    enabled: true
    group_name: Web            # le dossier affiché sur la page Moniteurs
    tags: { env: prod }
  - kind: snmp
    address: 10.0.0.2
    name: Switch cœur
    interval_secs: 60
    enabled: true
    parent: "http|https://example.org"   # une référence : type|adresse
    credential: { type: snmp_community, community: "${file:/run/secrets/snmp}" }

rules:
  - uid: site-lent
    name: Le site est lent
    kind: threshold
    query: dumbmonit_probe_duration_seconds
    operator: ">"
    threshold: 2
    for_secs: 300
    severity: warning
    selector: { kind: refs, refs: ["http|https://example.org"] }
    channels: [Téléphone]                # par nom de canal

rule_overrides:
  - rule_uid: cpu_high                   # une règle intégrée
    target: "snmp|10.0.0.2"
    threshold: 95

silences:
  - name: Sauvegardes de nuit
    schedule: { kind: weekly, days: [6], start_minute: 120, end_minute: 240, timezone: Europe/Paris }
    enabled: true

status_pages:
  - slug: maison
    title: Home lab
    published: true
    theme: default
    show_uptime_days: 90
    items:
      - { target: "http|https://example.org", label: Site web }
```

Les sections sont celles de la [sauvegarde](../install/backup.md), avec les mêmes
clés et les mêmes références : `GET /api/config/export` montre tous les champs.
Les objets se retrouvent par leur clé naturelle, jamais par un identifiant
interne : un moniteur par `kind` + `address`, une règle par `uid`, un canal par
`name`, une page de statut par `slug`, une fenêtre de maintenance par `name`
(et son moniteur, le cas échéant). Le fichier est donc portable d’une instance à
l’autre.

Hors fichiers : comptes et jetons, incidents, enrôlement des agents, carte du
monde, paquets. Les moniteurs créés par les [étiquettes Docker](docker-labels.md)
ou découverts par un agent ne sont pas exportés et un fichier ne peut pas les
déclarer. Les règles intégrées ne se déclarent pas non plus : ajustez-les avec
`rule_overrides`. Le mot de passe d’une page de statut privée se règle dans
l’interface et reste en place quand un fichier est appliqué.

## Secrets

Un fichier ne contient jamais de secret. Partout où il en faut (`credential` d’un
moniteur, `secrets` d’un canal), écrivez une référence :

| Référence | Sens |
|---|---|
| `${NOM}` | la variable d’environnement `NOM` |
| `${file:/chemin}` | le contenu d’un fichier, saut de ligne final retiré (secrets Docker ou Kubernetes) ; un chemin relatif part du dossier du fichier YAML |

Les références sont résolues là où se trouve le secret : par `dumbmonit-cli` sur
votre machine, ou par le serveur pour le dossier surveillé. L’API, elle,
**refuse** une référence restée telle quelle : un administrateur ne peut pas
faire lire au serveur son propre environnement. L’export remplace chaque secret
par une référence comme `${DUMBMONIT_CHANNEL_TELEPHONE_TOKEN}` et liste en tête
de fichier les variables à définir.

## Géré par fichier

Tout ce qu’un fichier déclare est marqué **Géré par fichier** : un badge dans les
listes, et l’API (donc l’interface) refuse de le modifier ou de le supprimer à la
main, car la prochaine application écraserait le changement sans prévenir. Un
moniteur peut être mis en pause. Pour reprendre la main :

- modifiez le fichier et appliquez-le de nouveau (le chemin normal) ;
- ou *Réglages → Système → Fichiers de configuration → Libérer*
  (`POST /api/config/release`) : tous les objets redeviennent modifiables, rien
  n’est supprimé, et la prochaine application reprend ceux qui sont déclarés.

Un objet existant qu’un fichier déclare est adopté : il apparaît dans le plan
comme une modification avec `managed_by_file`.

## Plan et application

`POST /api/config/plan` rend ce qui serait créé, modifié ou supprimé, en nommant
les champs qui diffèrent (jamais leurs valeurs), sans rien changer.
`POST /api/config/apply` l’exécute **en une seule transaction** : une référence
à un moniteur absent, une valeur que la base refuse, n’importe quoi, et rien
n’est écrit.

**Prune.** Par défaut, rien n’est jamais supprimé. Avec `prune`, les objets
qu’un fichier gère et qu’il ne déclare plus sont supprimés. Les objets créés à
la main ne sont jamais touchés, quelle que soit l’option.

```bash
dumbmonit-cli login --url https://monit.example.org      # demande un jeton d’API administrateur
dumbmonit-cli plan  config/ --check                      # code de sortie 2 si quelque chose changerait
dumbmonit-cli apply config/ --prune
dumbmonit-cli export -o current.yaml
```

Un dossier en argument applique tous ses fichiers `.yaml` / `.yml` comme un seul
document. Les deux commandes acceptent `--json`.

## L’outil en ligne de commande

`dumbmonit-cli` est un binaire unique, sans dépendance d’exécution. Compilez-le
depuis le dépôt (`cargo build --release -p dumbmonit-cli`).

| Commande | Effet |
|---|---|
| `login --url URL [--token T]` | vérifie la connexion et range l’adresse et le jeton dans `~/.config/dumbmonit/cli.json` (mode 600). `DUMBMONIT_URL` et `DUMBMONIT_TOKEN` fonctionnent aussi |
| `export [-o FICHIER]` | écrit l’instance dans un fichier |
| `plan FICHIER\|DOSSIER... [--prune] [--check]` | affiche le plan |
| `apply FICHIER\|DOSSIER... [--prune]` | applique les fichiers |
| `targets list` | liste les moniteurs |
| `silence create --name N --duration 2h [--target-id ID] [--match k=v] [--comment C]` | ouvre une fenêtre de maintenance qui commence maintenant |

Le jeton doit être celui d’un administrateur, avec la portée `write` (Réglages →
Jetons d’API). Toutes les commandes acceptent `--json` pour les scripts.

## Dossier surveillé

```yaml
services:
  dumbmonit:
    environment:
      DUMBMONIT_CONFIG_DIR: /config
      NTFY_TOKEN_FILE: /run/secrets/ntfy
    volumes:
      - ./dumbmonit-config:/config:ro
```

Tous les fichiers `.yaml` / `.yml` du dossier forment un seul document. Il est
appliqué avec `prune` au démarrage et à chaque changement d’un fichier (le
dossier est relu toutes les cinq secondes : un volume réseau ou un montage
Kubernetes fonctionne). Un dossier sans aucun fichier YAML n’applique rien : un
montage manquant ne doit pas supprimer ce que les fichiers géraient. Le dernier
résultat, ou la dernière erreur, s’affiche dans la section des réglages.

Les références d’un dossier surveillé utilisent l’environnement et les fichiers
du serveur.

## Référence

| Route | Rôle |
|---|---|
| `POST /api/config/plan` | le plan de `{ "yaml": "...", "prune": false }` (ou `"config": {...}` en JSON) |
| `POST /api/config/apply` | l’applique |
| `GET /api/config/export` | l’instance en YAML (`?format=json` pour le document et les variables) |
| `GET /api/config/status` | dossier surveillé et objets gérés |
| `POST /api/config/release` | rend tout à l’édition manuelle |

Réservé aux administrateurs (session ou jeton `write`). Voir la
[référence de l’API](../reference/api.md).

# Configuration

Tout passe par des variables d'environnement ; aucune n'est obligatoire. Il
n'y a pas de fichier de configuration à monter.

## Serveur

| Variable | Défaut | Rôle |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Adresse d'écoute de l'API et de l'interface web, à l'intérieur du conteneur. |
| `PUID` / `PGID` | `65532` | Utilisateur et groupe sous lesquels le serveur s'exécute lorsque le conteneur démarre sous root (`user: "0:0"`, comme dans le `docker-compose.yml` fourni) : le répertoire de données leur est transféré au démarrage, puis root est abandonné. `0` est refusé. `DUMBMONIT_PUID` / `DUMBMONIT_PGID` ont la priorité. Ignorés dans les autres cas. Voir [Permissions](../install/docker.md#permissions). |
| `DUMBMONIT_DATA_DIR` | `/data` | Répertoire persistant : `dumbmonit.db` (SQLite), `secret.key`, `vm/` (VictoriaMetrics embarqué) et `webchange/` (captures d'écran des changements de site). |
| `DUMBMONIT_VM_URL` | *(non défini)* | URL de base d'un VictoriaMetrics externe (`http://host:8428`). Quand elle est définie, celui embarqué n'est pas démarré ; voir [ci-dessous](#embedded-victoriametrics). |
| `DUMBMONIT_SECRET` | *(généré)* | Secret de l'instance. Si vide, lu depuis `<répertoire de données>/secret.key`, généré au premier démarrage. Chiffre les identifiants de surveillance, les secrets de canal et les jetons d'agent avec AES-256-GCM. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Sondes simultanées maximum, tous collecteurs confondus. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Délai maximal d'une sonde. Les surveillances de service ont leur propre option `timeout_seconds`, plus courte. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Période des écritures regroupées vers VictoriaMetrics. |
| `DUMBMONIT_FLUSH_BATCH` | `5000` | Nombre d'échantillons en tampon qui déclenche une écriture avant que la période ne s'écoule. |
| `DUMBMONIT_WORKERS` | `min(4, CPUs)` | Threads du runtime async. Le serveur passe son temps à attendre le réseau : quatre couvrent un homelab, augmentez seulement pour des centaines de surveillances. |
| `DUMBMONIT_DB_POOL` | `4` | Connexions SQLite ouvertes maximum. Chacune coûte un thread et un cache de page de 2 Mo ; les connexions inactives se ferment après cinq minutes. |
| `DUMBMONIT_LOG` | `info` | Filtre de journal, syntaxe `tracing` : `debug`, `warn`, `dumbmonit=debug,sqlx=warn`… |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Répertoire des binaires d'agent servis sous `/download/`. Vide ou manquant : la commande d'installation échoue avec `404`, le serveur tourne quand même. |
| `DUMBMONIT_RESET_PASSWORD` | *(désactivé)* | `1`, `true`, `yes` ou `on` : efface le mot de passe et chaque session au démarrage. L'interface montre alors `/setup` à nouveau. Retirez-la ensuite. |
| `DUMBMONIT_COOKIE_SECURE` | *(désactivé)* | `1` pour régler l'attribut `Secure` sur le cookie de session. Seulement derrière HTTPS : en HTTP simple le navigateur ne renverrait jamais le cookie. |
| `DUMBMONIT_TRUSTED_PROXIES` | *(vide)* | Reverse proxies dont le `X-Forwarded-For` est cru, comme adresses ou plages CIDR séparées par des virgules (`10.0.0.5, 172.16.0.0/12`). Le limiteur de débit de connexion et le journal de sécurité voient alors la vraie adresse client au lieu de celle du proxy ; sans cela, chaque visiteur derrière le proxy partage un seul compartiment. L'en-tête est ignoré depuis toute autre adresse, un client ne peut donc pas choisir son propre compartiment. |
| `DUMBMONIT_SCAN_SUBNETS` | *(vide)* | Réseaux proposés par le balayage du formulaire de surveillance, comme plages CIDR IPv4 séparées par des virgules (`192.168.1.0/24, 10.0.5.0/24`). En mode pont Docker, le serveur ne voit que le réseau Docker, pas votre réseau local : renseignez-la pour pré-remplir le balayage avec le bon. Non définie : le balayage propose les réseaux /24 des surveillances déjà surveillés, puis les réseaux du serveur (seulement hors conteneur), sinon laisse le champ vide. |
| `DUMBMONIT_OIDC_AUTO_CREATE` | *(désactivé)* | `true` pour créer un compte pour une identité inconnue à sa première connexion unique. Désactivé, seuls les comptes existants (déjà liés, ou retrouvés par un email vérifié par le fournisseur) peuvent se connecter. Lu seulement tant qu'aucun réglage de connexion unique n'est enregistré dans l'interface. |
| `DUMBMONIT_OIDC_ALLOW_HTTP` | *(désactivé)* | `1` pour accepter un émetteur OpenID Connect en `http://` simple. Désactivé, l'écran de réglages refuse tout sauf `https://` : un émetteur en clair livre le code d'autorisation et le secret client à qui écoute sur le réseau. Seulement pour un fournisseur de test sur le loopback. |
| `DUMBMONIT_METRICS_PUBLIC` | *(désactivé)* | `1` pour servir `/metrics`, `/federate` et `/prometheus/api/v1/…` sans jeton d'API. Quiconque atteint le port lit alors chaque mesure de chaque surveillance et l'état de l'instance ; ne le réglez que là où autre chose restreint déjà l'accès au port. Voir [Métriques](metrics.md#scraping-dumbmonit). |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Période d'évaluation des alertes. Les valeurs sous 10 sont relevées à 10. |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | `90` | Rétention de l'historique d'alertes, en jours. |
| `DUMBMONIT_BACKUP_ENABLED` | `1` | Sauvegardes locales planifiées de la base de données. `0`, `false`, `no` ou `off` pour arrêter de les écrire. Voir [Sauvegarde et restauration](../install/backup.md#scheduled-local-backups). |
| `DUMBMONIT_UPDATE_CHECK` | `1` | `off` arrête la vérification quotidienne de la dernière release GitHub (instances isolées). Voir [Mise à jour](../install/update.md). |
| `DUMBMONIT_BACKUP_DIR` | `<répertoire de données>/backups` | Où les sauvegardes planifiées sont écrites. Pointez-le sur un second volume pour survivre à la perte du premier. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Heures entre deux sauvegardes, de 1 à 8760. La première a lieu un intervalle après le démarrage, pas au démarrage. |
| `DUMBMONIT_DEMO` | *(désactivé)* | `1` pour une démo publique, en lecture seule, avec un parc fictif fixe, connexion `demo` / `demo`. **Recrée la base de données à chaque démarrage** : jamais sur un vrai volume de données. Voir [Lancer une instance de démonstration](../install/demo.md). |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Combien de sauvegardes sont conservées, de 1 à 365. Les plus anciennes sont supprimées avec leur `.key`. |
| `DUMBMONIT_BROWSER_URL` | *(non défini)* | Chromium headless utilisé pour les captures d'écran des [changements de site web](../devices/webchange.md#screenshots), sous la forme `http://host:9222` (son port DevTools). Ajoutez un service `chromedp/headless-shell` à votre fichier Compose, comme montré sous [Captures d'écran](../devices/webchange.md#screenshots). Non défini : comparaison de texte seulement. |

La rétention de référence (60 jours) et la période d'apprentissage de 14
jours ne sont pas configurables.

### Anciens noms `EZYMONIT_*`

DumbMonit s'appelait EzyMonit jusqu'en septembre 2026. Chaque variable
`DUMBMONIT_*` est aussi lue sous son ancien nom `EZYMONIT_*`, en repli, avec
un avertissement au démarrage par variable encore réglée ainsi
(`EZYMONIT_X is deprecated, use DUMBMONIT_X`). L'ancien fichier de base de
données `/data/ezymonit.db` est renommé en `dumbmonit.db` au démarrage, et
les jetons d'agent `ezym_…` restent valides (les nouveaux sont `dmon_…`).
Le cookie de session est maintenant `dumbmonit_session` : le premier
démarrage après le renommage déconnecte tout le monde une fois.

### VictoriaMetrics embarqué {#embedded-victoriametrics}

L'image fournit le binaire VictoriaMetrics. Sauf si `DUMBMONIT_VM_URL` est
réglée, le serveur le démarre comme processus enfant au démarrage, répercute
ses lignes de journal dans le journal du serveur, le redémarre avec un
backoff s'il meurt, et l'arrête à l'extinction (`SIGTERM`, 20 s de grâce,
puis kill ; le `stop_grace_period` du fichier Compose laisse la place pour
ça).

| Variable | Défaut | Rôle |
|---|---|---|
| `DUMBMONIT_VM_BINARY` | `/victoria-metrics-prod` | Chemin du binaire VictoriaMetrics à démarrer. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Adresse d'écoute, à l'intérieur du conteneur. Loopback par défaut : seul le serveur lui parle. La surcouche de développement règle `0.0.0.0:8428` et publie le port. |
| `DUMBMONIT_VM_RETENTION` | `12` | Rétention, dans la syntaxe de `-retentionPeriod` : un nombre de mois, ou une durée comme `30d` ou `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Budget pour les caches de VictoriaMetrics (`-memory.allowedBytes`). Sans budget, VM les dimensionne pour 60 % de la RAM de l'hôte et sa mémoire résidente grandit pendant des jours. Quelques dizaines de surveillances tiennent dans 256 Mo ; augmentez pour des centaines d'hôtes. |

Ses données vivent dans `<répertoire de données>/vm`, sur le même volume
que la base de données. Trois options sont fixées par le serveur et non
configurables : `-search.maxUniqueTimeseries=100000` (une requête
incontrôlée ne peut pas épuiser le budget), `-search.maxConcurrentRequests=4`
(une interface plus le moteur d'alertes) et `-dedup.minScrapeInterval=10s`
(les points en double à moins de dix secondes d'écart sont fusionnés ; rien
n'est perdu à la cadence de 30 s des collecteurs).

Avec `DUMBMONIT_VM_URL` réglée, rien de tout cela ne s'applique : le
serveur utilise l'instance que vous nommez et celui embarqué n'est jamais
démarré. `GET /api/health` dit dans quel cas vous êtes
(`victoria.embedded`).

## Réglages au niveau Compose

Le `docker-compose.yml` fourni ne lit aucune variable et n'a besoin d'aucun
fichier `.env`. Tout se règle sous `environment:` du service ou en modifiant
le fichier ; voir [Options avancées](../install/docker.md#advanced-options)
(port, secret, rétention et mémoire, VictoriaMetrics externe, réinitialisation
du mot de passe, captures d'écran). La surcouche de développement
`docker-compose.dev.yml` lit `DUMBMONIT_VM_PORT` (défaut `8428`) pour le port
de l'hôte du VictoriaMetrics embarqué.

## Ports

| Port | Conteneur | Usage |
|---|---|---|
| `8080/tcp` | `dumbmonit` | Interface web, API, installateurs et téléchargements d'agent, ingestion d'agent. Le seul port à publier. |
| `8428/tcp` | `dumbmonit` | API HTTP du VictoriaMetrics embarqué, sur le loopback du conteneur. Publié seulement par la surcouche de développement. |

Sortant : SNMP (UDP 161 par défaut) vers les surveillances, HTTPS vers Proxmox,
PBS, Synology et les services de notification, plus ce que vos surveillances de
service ciblent. Le ping ICMP a besoin du sysctl
`net.ipv4.ping_group_range` réglé dans le fichier Compose (aucune
capacité).

## Volumes et fichiers

Un volume, `dumbmonit-data`, monté sur `/data`. Son nom est fixé dans le
fichier Compose, quel que soit le nom du projet.

| Chemin (dans le conteneur) | Volume | Contenu |
|---|---|---|
| `/data/dumbmonit.db` | `dumbmonit-data` | Surveillances, identifiants (chiffrés), règles, état et historique des alertes, références, silences, canaux, jetons d'agent, hash de mot de passe, sessions. |
| `/data/secret.key` | `dumbmonit-data` | Le secret de l'instance. **Sauvegardez-le.** Sans lui, les identifiants chiffrés sont irrécupérables et le serveur refuse de démarrer. |
| `/data/backups/` | `dumbmonit-data` | Sauvegardes locales planifiées : `dumbmonit-<timestamp>.db` (une copie SQLite en ligne) et le `.key` correspondant. Quotidien par défaut, sept conservées. |
| `/data/vm/` | `dumbmonit-data` | Séries temporelles de VictoriaMetrics embarqué, 12 mois par défaut. Vide quand `DUMBMONIT_VM_URL` est réglée. |
| `/agents/` | image | Binaires d'agent. Montez un autre répertoire et réglez `DUMBMONIT_AGENT_DIR` pour fournir les vôtres. |
| `/etc/ssl/certs/ca-certificates.crt` | image | Racines TLS pour le HTTPS sortant. Montez votre propre ensemble ici pour faire confiance à une autorité privée. |

## Réinitialiser le mot de passe

```bash
# ajoutez DUMBMONIT_RESET_PASSWORD: "1" sous environment: dans docker-compose.yml
docker compose up -d
# ouvrez l'interface : /setup demande un nouveau mot de passe
# retirez de nouveau la ligne, puis :
docker compose up -d
```

La réinitialisation efface le mot de passe et chaque session ; surveillances,
règles et canaux restent intacts.

## Agent

L'agent a ses propres variables et fichier de configuration : voir
[Agent Linux, macOS, FreeBSD et Windows](../devices/agent.md#configuration-file).

### Portée de la collecte

Ce que l'agent mesure, et donc combien de séries une machine produit. Les
valeurs par défaut gardent un hôte avec des dizaines de conteneurs à
quelques centaines de séries ; chaque clé est une entrée de `agent.yaml`,
surchargeable par sa variable d'environnement.

| Clé | Variable | Défaut | Rôle |
|---|---|---|---|
| `interfaces_ignore` | `DUMBMONIT_AGENT_INTERFACES_IGNORE` | `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` | Interfaces réseau laissées de côté : interfaces de conteneur et de VM, le loopback. Une liste de noms exacts ou d'expressions régulières (non ancrées ; tout ce qui a un métacaractère est une regex). La valeur remplace le défaut ; une liste vide garde chaque interface. |
| `interfaces_only` | `DUMBMONIT_AGENT_INTERFACES_ONLY` | *(vide)* | Liste d'autorisation, même syntaxe. Quand elle est réglée, seules ces interfaces sont rapportées et `interfaces_ignore` n'est pas consultée. |
| `mounts_ignore` | `DUMBMONIT_AGENT_MOUNTS_IGNORE` | `^/(var/lib/docker/\|run/(user\|docker\|containerd\|snapd\|credentials\|systemd\|lock\|udev\|netns\|lxc\|lxd)/\|sys/\|proc/\|dev/\|snap/)` | Points de montage laissés de côté de l'usage du système de fichiers et des E/S disque, même syntaxe. Les disques amovibles sous `/run/media/` restent. Les pseudo-systèmes de fichiers (`overlay`, `tmpfs`, `squashfs`, `fuse.portal`, `cgroup2`…) sont toujours ignorés, quel que soit le chemin. |
| `cpu_per_core` | `DUMBMONIT_AGENT_CPU_PER_CORE` | `false` | Une série `cpu_core_usage_percent` par cœur, en plus de l'usage global. `cpu_count` est rapporté dans les deux cas. |
| `docker_max_containers` | `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | `200` | Conteneurs détaillés par cycle, ceux qui tournent d'abord. Au-delà, les conteneurs sont seulement comptés (`container_count`, `container_running_count` restent exacts) et `container_series_skipped` dit combien. `0` compte sans détailler. |

Dans l'environnement, les listes sont séparées par des virgules
(`^veth, ^br-, docker0`) ; en YAML, soit une liste, soit une chaîne séparée
par des virgules. Les compteurs d'E/S disque (`disk_read_bytes`,
`disk_written_bytes`) portent seulement le label `device` : une série par
périphérique bloc, quel que soit le nombre de fois où il est monté.

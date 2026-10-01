# Agent système DumbMonit

Petit binaire installé sur la machine à surveiller. Il mesure le processeur, la
mémoire, les disques, le réseau, les services et les conteneurs, puis **pousse**
ses relevés vers le serveur DumbMonit en HTTP.

Le sens de connexion est délibéré : c'est l'agent qui appelle le serveur, jamais
l'inverse. Il traverse donc les NAT et les pare-feux domestiques, et n'impose
d'ouvrir **aucun port** sur la machine surveillée.

## Installation

### Linux (systemd, OpenRC), macOS (launchd), FreeBSD (rc.d)

```sh
curl -sSL http://serveur:8080/install.sh | sudo sh -s -- --token=dmon_xxx --url=http://serveur:8080
```

Le script choisit le binaire d'après `uname -s` et `uname -m`. Celui de macOS
n'est pas dans l'image (le SDK d'Apple ne se redistribue pas) : le serveur
renvoie vers la pièce jointe de la dernière publication GitHub, et le script
suit le renvoi.

### Windows (service)

```powershell
& ([scriptblock]::Create((irm http://serveur:8080/install.ps1))) -Token dmon_xxx -Url http://serveur:8080
```

Le jeton s'obtient dans l'interface, ou par l'API :

```sh
curl -X POST http://serveur:8080/api/agent/tokens \
     -H 'content-type: application/json' \
     -d '{"name":"parc maison","base_url":"http://serveur:8080"}'
```

La réponse contient le jeton en clair **une seule fois** : le serveur n'en garde
que l'empreinte. Elle contient aussi les deux commandes d'installation toutes
faites.

## Configuration

Fichier YAML, `/etc/dumbmonit/agent.yaml` sur Linux,
`/usr/local/etc/dumbmonit/agent.yaml` sur macOS et FreeBSD,
`C:\ProgramData\DumbMonit\agent.yaml` sur Windows :

```yaml
server_url: http://serveur:8080
token: dmon_...
interval_secs: 30          # période d'échantillonnage
hostname: nas-cave         # facultatif : nom annoncé au serveur
services:                  # unités systemd ou services Windows
  - sshd
  - docker
tags:                      # étiquettes libres, préfixées « tag_ » côté serveur
  role: nas
  salle: cave
docker: true               # inventaire des conteneurs (défaut : true)
docker_socket: /var/run/docker.sock
max_buffered_samples: 20000
log_level: info
```

Chaque clé se surcharge par l'environnement, ce qui rend l'agent utilisable en
conteneur sans monter de fichier :

| Variable | Effet |
| --- | --- |
| `DUMBMONIT_AGENT_CONFIG` | Chemin du fichier de configuration |
| `DUMBMONIT_AGENT_URL` | URL du serveur |
| `DUMBMONIT_AGENT_TOKEN` | Jeton d'enregistrement |
| `DUMBMONIT_AGENT_INTERVAL_SECS` | Période d'échantillonnage |
| `DUMBMONIT_AGENT_HOSTNAME` | Nom annoncé au serveur |
| `DUMBMONIT_AGENT_SERVICES` | Services à surveiller, séparés par des virgules |
| `DUMBMONIT_AGENT_TAGS` | `cle=valeur`, séparés par des virgules |
| `DUMBMONIT_AGENT_DOCKER` | `true` / `false` |
| `DUMBMONIT_AGENT_DOCKER_SOCKET` | Chemin du socket Docker |
| `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | Conteneurs détaillés par hôte, `200` par défaut (`0` : décomptes seuls) |
| `DUMBMONIT_AGENT_INTERFACES_IGNORE` | Interfaces ignorées (noms ou regex, séparés par des virgules) ; par défaut `^(veth|br-|docker|virbr|lo$|vEthernet)` |
| `DUMBMONIT_AGENT_INTERFACES_ONLY` | Interfaces à garder ; remplace la liste d'exclusion quand elle est définie |
| `DUMBMONIT_AGENT_MOUNTS_IGNORE` | Points de montage exclus des systèmes de fichiers et des E/S disque |
| `DUMBMONIT_AGENT_CPU_PER_CORE` | `true` pour envoyer aussi une série par cœur (`false` par défaut) |
| `DUMBMONIT_AGENT_MAX_BUFFERED_SAMPLES` | Taille du tampon de reprise |
| `DUMBMONIT_AGENT_LOG` | `trace`, `debug`, `info`, `warn`, `error` |

## Diagnostic

```sh
dumbmonit-agent --dry-run          # affiche les mesures, n'envoie rien
dumbmonit-agent --once             # envoie un seul lot puis s'arrête
journalctl -u dumbmonit-agent -f   # journal du service
```

Le jeton n'apparaît jamais dans les journaux, pas même tronqué, y compris dans
les messages d'erreur du client HTTP.

## Ce que l'agent remonte

| Famille | Métriques | Étiquettes |
| --- | --- | --- |
| Processeur | `cpu_usage_percent`, `cpu_core_usage_percent`, `cpu_count`, `load_average_{1,5,15}` | `core` |
| Mémoire | `memory_{total,used,available}_bytes`, `memory_used_percent`, `swap_{total,used}_bytes`, `swap_used_percent` | — |
| Systèmes de fichiers | `filesystem_{total,used,free}_bytes`, `filesystem_used_percent` | `mountpoint`, `device`, `fstype` |
| Réseau (compteurs) | `if_octets_{in,out}`, `if_packets_{in,out}`, `if_errors_{in,out}` | `ifname` |
| Disques (compteurs) | `disk_read_bytes`, `disk_written_bytes`, une série par périphérique | `device` |
| Hôte | `uptime_seconds`, `process_count` | — |
| Services | `service_up` (1 = en marche) | `service` |
| Conteneurs | `container_up`, `container_count`, `container_running_count` | `container`, `image` |
| Agent | `agent_collect_seconds`, `agent_buffered_samples`, `agent_dropped_samples` | — |

Les compteurs cumulatifs partent **bruts**, en `Counter` : le taux est calculé à
la lecture. Un agent qui calculerait lui-même des débits devrait garder un état
entre deux cycles, et c'est cet état qui mentirait au premier redémarrage.

Les étiquettes d'identité (`target`, `host`, `tag_*`) sont posées par le serveur
à la réception, jamais par l'agent : une machine ne peut donc pas écrire dans les
séries d'une autre, même en forgeant ses étiquettes.

## Comportement en cas de coupure

Quand le serveur est injoignable, l'agent continue de mesurer et garde ses
relevés en mémoire, avec leur horodatage d'origine. Ils repartent dès que la
liaison revient : un redémarrage de serveur ne creuse pas de trou dans les
graphes.

Le tampon est borné (`max_buffered_samples`, environ une heure par défaut) et
sacrifie les mesures les plus anciennes quand il déborde — l'outil de
surveillance ne doit jamais devenir la panne.

Les tentatives d'envoi sont espacées par un délai qui double à chaque échec, de 5
secondes à 5 minutes, avec une part aléatoire : cent agents ne se jettent pas
tous sur le serveur à la milliseconde où il redémarre.

## Compilation

### Poste de développement

```sh
cargo build -p dumbmonit-agent
cargo test  -p dumbmonit-agent
cargo clippy -p dumbmonit-agent --all-targets -- -D warnings
```

### Binaires distribués

Six binaires, un par système, chacun avec son empreinte `<nom>.sha256` (format
de `sha256sum`) :

| Système | Fichier | Construit par |
|---|---|---|
| Linux x86_64, statique (musl) | `dumbmonit-agent-linux-x86_64` | étape `agent` du `Dockerfile` |
| Linux aarch64, statique (musl) | `dumbmonit-agent-linux-aarch64` | étape `agent` du `Dockerfile` |
| Windows x86_64 | `dumbmonit-agent-windows-x86_64.exe` | étape `agent` du `Dockerfile` |
| FreeBSD x86_64 | `dumbmonit-agent-freebsd-x86_64` | étape `agent` du `Dockerfile` |
| macOS Apple silicon | `dumbmonit-agent-macos-aarch64` | exécuteur macOS de `release.yml` |
| macOS Intel | `dumbmonit-agent-macos-x86_64` | exécuteur macOS de `release.yml` |

L'étape `agent` compile les quatre premiers sur la machine de construction,
avec cargo-zigbuild (zig sert d'éditeur de liens pour chaque cible) et une
chaîne Rust épinglée. L'image du serveur les embarque et les sert sur
`/download/…` ; l'étape `agent-dist` les sort seuls :

```sh
docker buildx build --target agent-dist --output type=local,dest=dist/agent .
# Sur une machine partagée, brider cargo : --build-arg CARGO_BUILD_JOBS=4
```

macOS n'y est pas, et ne peut pas y être : l'édition de liens réclame le SDK
d'Apple, que sa licence interdit de redistribuer. Les deux binaires sont
construits sur un exécuteur macOS (job `macos-agent`). Sur `main`, tous les
binaires restent trente jours en artefacts du workflow Release
(`dumbmonit-agent-binaries`, `dumbmonit-agent-macos`) ; sur une étiquette `v*`,
le job `github-release` attache les six et leurs empreintes à la publication
GitHub, vers laquelle le serveur renvoie pour `/download/dumbmonit-agent-macos-…`.

### Vérifier le code pour les autres systèmes

La chaîne d'intégration ne passe clippy que sous Linux. L'image
`ghcr.io/rust-cross/cargo-zigbuild` fournit les cibles macOS, Windows et
FreeBSD, et un compilateur C pour chacune (zig), que demandent `ring` et
`aws-lc-sys` ; clippy ne lie rien. Sa chaîne Rust est trop ancienne pour
`sysinfo` : en installer une récente, puis, pour chaque cible :

```sh
rustup toolchain install stable --profile minimal -c clippy \
  --target x86_64-apple-darwin,aarch64-apple-darwin,x86_64-pc-windows-gnu,x86_64-unknown-freebsd
cargo-zigbuild clippy -p dumbmonit-agent --all-targets --target x86_64-apple-darwin -- -D warnings
```

Le code propre à un système est isolé derrière `#[cfg(...)]` :
`collect/services.rs`, `collect/docker.rs`, `collect/sensors.rs`,
`collect/perf_counters.rs`, `identity.rs`, `shutdown.rs`, et `winsvc.rs` (point
d'entrée du service Windows, journal dans `agent.log` à côté de la
configuration, voir `logfile.rs`).

### À la main

```sh
# Sur le Mac lui-même, avec Rust :
cargo build --release -p dumbmonit-agent
# Sous Windows, avec les outils de compilation Visual Studio :
cargo build --release -p dumbmonit-agent --target x86_64-pc-windows-msvc
```

## Empreinte

Runtime Tokio mono-fil, journalisation sans moteur d'expressions régulières,
rafraîchissement système limité à ce qui est publié, et pas de client Docker
dédié — le dialogue HTTP sur le socket tient en quelques dizaines de lignes.
L'unité systemd installée pose en plus `MemoryMax=128M` et `CPUQuota=20%` : même
un défaut de l'agent ne peut pas emporter la machine qu'il surveille.

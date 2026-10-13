# Agent Linux, macOS, FreeBSD et Windows

Un petit binaire installé sur la machine à surveiller. Il mesure CPU,
mémoire, disques, réseau, services et conteneurs — et, là où la machine
les expose, températures, santé de disque et pools ZFS — puis **envoie**
ses relevés au serveur DumbMonit via HTTP.

La direction est délibérée : l'agent appelle le serveur, jamais l'inverse.
Il traverse NAT et pare-feux domestiques, et ne demande d'ouvrir **aucun
port** sur la machine surveillée.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_`. Les labels d'identité
(`target`, `host`, `tag_*`) sont réglés par le serveur à la réception,
jamais par l'agent : une machine ne peut pas écrire dans les séries d'une
autre, même en falsifiant ses labels.

| Famille | Métriques | Labels |
|---|---|---|
| CPU | `cpu_usage_percent`, `cpu_count`, `load_average_1/5/15` ; `cpu_core_usage_percent` seulement avec `cpu_per_core: true` | `core` |
| Mémoire | `memory_total/used/available_bytes`, `memory_used_percent`, `swap_total/used_bytes`, `swap_used_percent` | |
| Systèmes de fichiers | `filesystem_total/used/free_bytes`, `filesystem_used_percent` | `mountpoint`, `device`, `fstype` |
| Réseau (compteurs) | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out` | `ifname` |
| Disques (compteurs) | `disk_read_bytes`, `disk_written_bytes`, une série par périphérique bloc | `device` |
| Hôte | `uptime_seconds`, `process_count` | |
| Services | `service_up` (1 = en cours) | `service` |
| Conteneurs | `container_up`, `container_count`, `container_running_count`, `container_series_skipped` (conteneurs au-delà de `docker_max_containers`) | `container`, `image` |
| Capteurs | `agent_sensor_temperature_celsius`, `agent_sensor_temperature_critical_celsius` (le seuil que le matériel déclare lui-même), `agent_sensor_fan_rpm` | `sensor`, `fan` |
| Santé de disque (SMART) | `agent_disk_smart_ok` (1 = le disque réussit sa propre auto-évaluation), `agent_disk_temperature_celsius`, `agent_disk_power_on_hours`, `agent_disk_wearout_percent`, `agent_disk_reallocated_sectors`, `agent_disk_pending_sectors`, `agent_disk_media_errors` | `device`, `model` |
| ZFS | `agent_zfs_pool_health` (0 online, 1 dégradé, 2 pire), `agent_zfs_pool_size/used/free_bytes`, `agent_zfs_pool_used_percent`, `agent_zfs_pool_fragmentation_percent`, `agent_zfs_pool_device_errors`, `agent_zfs_pool_data_errors`, `agent_zfs_pool_scrub_running`, `agent_zfs_pool_scrub_errors`, `agent_zfs_pool_scrub_age_seconds` | `pool` |
| WireGuard | `agent_wireguard_interface_peers` ; par pair `agent_wireguard_peer_handshake_age_seconds`, `agent_wireguard_peer_has_handshake`, `agent_wireguard_peer_keepalive_seconds`, `agent_wireguard_peer_rx_bytes`, `agent_wireguard_peer_tx_bytes` (compteurs) — voir [Tunnels WireGuard](#wireguard-tunnels) | `interface`, `peer`, `allowed_ips`, `name` |
| restic / Borg | `agent_backup_repo_reachable`, `agent_backup_repo_last_snapshot_age_seconds`, `agent_backup_repo_last_snapshot_bytes` (restic) — voir [Sauvegardes restic et Borg](#restic-and-borg-backups) | `tool`, `repo` |
| Compteurs de performance Windows | `agent_perf_counter`, une série par compteur listé dans `perf_counters` | `counter` |
| MDaemon (Windows) | `mdaemon_queue_messages`, `mdaemon_queue_frozen`, `mdaemon_sessions_active`, `mdaemon_sessions_total`, `mdaemon_messages_total`, `mdaemon_filtered_messages_total`, `mdaemon_server_active`, `mdaemon_running`, `mdaemon_uptime_seconds` — voir [MDaemon](mdaemon.md#mail-queues-through-the-agent) | `queue`, `protocol`, `filter`, `verdict`, `server` |
| Hyper-V (Windows) | `hyperv_vms_health_ok`, `hyperv_vms_health_critical`, `hyperv_host_cpu_percent`, `hyperv_host_cpu_guest_percent`, `hyperv_partitions` ; par VM `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes`, `hyperv_vm_memory_pressure_percent` ; par disque virtuel `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second`, `hyperv_vhd_latency_seconds`, `hyperv_vhd_queue_length`, `hyperv_vhd_errors_total` — voir [Hôtes Hyper-V](#hyper-v-hosts) | `vm`, `disk` |
| Agent | `agent_collect_seconds`, `agent_buffered_samples`, `agent_dropped_samples` | |

Une série qui ne peut pas être mesurée est **absente**, jamais zéro : pas
de `smartctl`, pas de `agent_disk_*` ; pas de sonde lisible, pas de
`agent_sensor_*` ; pas de ZFS, pas de `agent_zfs_*`. Une température hors
1–150 °C est abandonnée plutôt que publiée, et une lecture de ventilateur
à 0 tr/min est abandonnée aussi — une carte mère a plus de connecteurs de
ventilateur que de ventilateurs, et un vide lit zéro comme un ventilateur
mort.

### Ce qui est collecté où

| | Linux | macOS | FreeBSD | Windows |
|---|---|---|---|---|
| CPU, mémoire, swap, systèmes de fichiers, réseau, uptime, processus | oui | oui | oui | oui |
| Moyenne de charge | oui | oui | oui | non (Windows n'a pas cette notion) |
| Services | systemd (`systemctl`) | launchd (`launchctl`) | rc.d (`service`) | services Windows |
| Conteneurs Docker | oui | oui, si `docker_socket` pointe vers le socket de Docker Desktop | non (pas de démon Docker) | non |
| Températures | `/sys/class/hwmon` | capteurs SMC | `dev.cpu.N.temperature` (CPU seulement) | non |
| Ventilateurs | `/sys/class/hwmon` | non | non | non |
| Santé de disque (SMART) | `smartctl` | `smartctl` (Homebrew) | `smartctl` (`sysutils/smartmontools`) | `smartctl` |
| Pools ZFS | `zpool` (OpenZFS) | non | `zpool` | non |
| Santé de l'OS (mises à jour, redémarrage, unités en échec, SELinux) | oui | non | non | non |
| Sauvegardes Plakar | oui | oui | oui | oui |
| Tunnels WireGuard | `wg show` | si `wg` est installé (non testé) | si `wg` est installé (non testé) | non |
| Sauvegardes restic et Borg | oui | oui | oui | restic seulement (non testé ; Borg n'a pas de build Windows) |
| Compteurs de performance, files MDaemon, Hyper-V | non | non | non | oui |
| Identité machine | `/etc/machine-id` | `IOPlatformUUID` | `kern.hostuuid`, `/etc/hostid` | nom d'hôte |
| Mode relais | oui | oui | oui | oui |

Windows n'a pas de ligne de température volontairement : lire des capteurs
là demande de passer par WMI, avec des droits administratifs, pour des
valeurs que la plupart des machines n'exposent pas du tout. Ne rien
publier est la réponse honnête.

`smartctl` (smartmontools) et `zpool` sont recherchés à chaque période de
lecture et utilisés quand présents ; ni l'un ni l'autre n'est installé par
l'agent, et ni l'un ni l'autre n'est requis. Les deux ont besoin de root —
le service que l'installateur enregistre tourne en root, un agent démarré
à la main sous votre propre compte ne trouvera rien.

Les compteurs sont envoyés bruts, pour qu'un redémarrage de l'agent ne
produise jamais un faux débit ; appliquez `rate()` dans les requêtes.

Règles intégrées qui s'appliquent : Appareil inaccessible, CPU élevé, CPU
inhabituel. « Appareil inaccessible » fonctionne aussi sur les agents : le
serveur vérifie l'ancienneté du dernier lot reçu et déclare la machine
silencieuse après trois périodes manquées (90 secondes au moins).

Quatre autres se déclenchent seulement sur les machines qui produisent la
série correspondante, elles n'ont donc besoin d'aucune configuration et ne
peuvent jamais se déclencher sur une machine sans le matériel :

| Règle | Se déclenche quand |
|---|---|
| **Disque SMART en échec** (critique) | la propre auto-évaluation d'un disque le rapporte en échec |
| **Pool ZFS dégradé** (critique) | un pool n'est plus `ONLINE` depuis cinq minutes |
| **Scrub ZFS a trouvé des erreurs** (critique) | le dernier scrub a trouvé des erreurs, ou le pool rapporte des fichiers corrompus de façon permanente |
| **Température au-dessus du critique** (critique) | un capteur dépasse le seuil critique que **son propre matériel déclare** — aucune valeur n'est codée en dur, 85 °C étant une urgence sur un disque et un après-midi normal sur le CPU d'un portable |

Trois autres surveillent ce que vous déclarez ou lancez vous-même, et
restent silencieuses sinon : **Pair WireGuard silencieux** (un pair avec
un keepalive persistant et aucun handshake depuis 15 minutes),
**Sauvegarde restic/Borg trop ancienne** (aucun snapshot depuis 48 heures)
et **Dépôt restic/Borg inaccessible** — voir
[Tunnels WireGuard](#wireguard-tunnels) et
[Sauvegardes restic et Borg](#restic-and-borg-backups).

Sur un serveur Windows qui fait tourner MDaemon, trois autres surveillent
ses files : **File de mail MDaemon qui grandit**, **File Bad MDaemon non
vide** et **File Retry MDaemon élevée** (voir
[Règles](../alerting/rules.md#mdaemon-and-securitygateway)). Sur un hôte
Hyper-V, trois surveillent les machines virtuelles : **Santé de VM
Hyper-V critique**, **CPU d'hôte Hyper-V élevé** et **Erreurs de disque
virtuel Hyper-V** (voir [Hôtes Hyper-V](#hyper-v-hosts)).

## Installer {#install}

Les étapes ci-dessous sont celles que montre l'avis à côté du formulaire.
Il n'y a aucun compte à créer sur la machine : le jeton d'enrôlement est
le seul secret.

1. Enregistrez l'appareil dans DumbMonit : un jeton d'enrôlement (dmon_…) est montré une fois, avec la commande d'installation prête à copier pour Linux et pour Windows. (Un jeton peut aussi être créé dans **Réglages → Agents**.)
2. Ce jeton est la clé de l'agent pour envoyer ses mesures à DumbMonit. Ce n'est pas un compte sur la machine : rien à créer là. Un jeton créé depuis le formulaire d'appareil est à usage unique : il enrôle cette seule machine et rien d'autre. Pour un parc, créez un jeton réutilisable dans Réglages → Agents.
3. Lancez la commande d'installation sur la machine à surveiller, avec des droits élevés (sudo sur Linux et macOS, un PowerShell élevé sur Windows). Elle télécharge l'agent, écrit le jeton dans agent.yaml et démarre le service.
4. La machine apparaît d'elle-même en quelques secondes, nommée d'après son nom d'hôte. À ce premier lot, le serveur donne à l'agent un secret qui lui est propre et le stocke dans `/etc/dumbmonit/agent-secret`, lisible par personne d'autre : à partir de là, cette machine est la seule qui peut rapporter sous cet appareil. Jeton perdu ? Réglages → Agents vous laisse le révoquer et en créer un autre.
5. Docker : pour voir les conteneurs et laisser DumbMonit les redémarrer ou les mettre à jour, l'agent doit atteindre le socket Docker. Le service que l'installateur enregistre le peut déjà ; si vous faites tourner l'agent sous un utilisateur dédié à la place, ajoutez cet utilisateur au groupe « docker » et redémarrez-le. Les politiques de redémarrage et de mise à jour automatique se règlent alors par conteneur sur la page de l'appareil.

Deux choses valent d'être lues avant de déployer ça sur plus d'une
machine : [Liaison](#binding-one-machine-one-agent), ce qui fait qu'un
appareil appartient vraiment à une installation d'agent, et
[Usage unique ou parc](#single-use-or-fleet), le choix que vous faites
quand vous créez un jeton.

    ```
    usermod -aG docker dumbmonit
    ```

6. Sauvegardes Plakar : détectées automatiquement — le panneau Backups apparaît quand le binaire plakar ou un kloset (~/.config/plakar/stores.yml de chaque utilisateur, ~/.plakar, /var/lib/plakar) est trouvé, et rien n'est montré sinon. Réglez "plakar_klosets" dans agent.yaml pour surveiller une liste fixe, ou "plakar: false" pour vous retirer.

!!! note
    L'agent contacte le serveur, jamais l'inverse : aucun port n'a besoin
    d'être ouvert sur la machine surveillée.

=== "Linux (systemd ou OpenRC)"

    ```sh
    curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Le script est en `sh` POSIX (BusyBox et dash fonctionnent), télécharge
    le binaire pour l'architecture de la machine (x86_64 ou aarch64) depuis
    le serveur, écrit `/etc/dumbmonit/agent.yaml` (mode 0600) et enregistre
    `dumbmonit-agent` avec systemd, ou avec OpenRC sur Alpine. L'unité
    systemd est renforcée (`ProtectSystem=strict`, `NoNewPrivileges`,
    `MemoryMax=128M`, `CPUQuota=20%`).

=== "macOS (launchd)"

    La même commande que sur Linux, lancée avec `sudo` :

    ```sh
    curl -sSL http://server:8080/install.sh | sudo sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    Les binaires macOS ne sont **pas** dans l'image DumbMonit : les
    construire demande le SDK d'Apple, dont la licence interdit la
    redistribution. Ils sont attachés à chaque
    [release GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
    à la place — `dumbmonit-agent-macos-aarch64` (Apple silicon) et
    `dumbmonit-agent-macos-x86_64` (Intel), chacun avec son `.sha256` — et
    le serveur redirige `/download/dumbmonit-agent-macos-…` là, le script
    choisit donc celui pour l'architecture du Mac (`uname -m`). Le serveur
    ne fournit pas ces binaires, la commande d'installation ne porte donc
    aucune somme de contrôle pour eux : ajoutez
    `--sha256=macos-aarch64:<hex>` (ou `macos-x86_64`) avec la valeur du
    `.sha256` de la release, sinon l'installateur refuse le
    téléchargement. Le Mac a aussi besoin d'atteindre github.com.

    Sans cet accès, ou pour installer un binaire que vous avez déjà,
    téléchargez-le et passez-le avec `--bin` (comparez-le d'abord avec le
    `.sha256` de la release : `--bin` installe ce que vous donnez) :

    ```sh
    curl -sSLO https://github.com/laupernoe/dumbmonit/releases/latest/download/dumbmonit-agent-macos-aarch64
    curl -sSL http://server:8080/install.sh | sudo sh -s -- \
        --token=dmon_xxx --url=http://server:8080 --bin=./dumbmonit-agent-macos-aarch64
    ```

    Le script installe le binaire dans `/usr/local/bin` (et retire
    l'attribut de quarantaine qu'un téléchargement par navigateur porte,
    qui ferait refuser à Gatekeeper de le démarrer), écrit
    `/usr/local/etc/dumbmonit/agent.yaml` (mode 0600) et enregistre le
    démon système `com.dumbmonit.agent` dans `/Library/LaunchDaemons`,
    démarré avec `launchctl bootstrap system`. Il tourne au démarrage,
    sans personne connecté.

    !!! note "Vous préférez le construire vous-même ?"
        Sur le Mac lui-même, avec Rust installé :
        `cargo build --release -p dumbmonit-agent`, puis
        `--bin=target/release/dumbmonit-agent`. C'est le même binaire que
        la tâche de release produit.

=== "FreeBSD (rc.d)"

    ```sh
    fetch -qo - http://server:8080/install.sh | sh -s -- --token=dmon_xxx --url=http://server:8080
    ```

    x86_64 seulement. Le script télécharge `dumbmonit-agent-freebsd-x86_64`
    depuis le serveur, écrit `/usr/local/etc/dumbmonit/agent.yaml` (mode
    0600) et installe `/usr/local/etc/rc.d/dumbmonit_agent`, activé avec
    `service dumbmonit_agent enable`. `daemon(8)` supervise l'agent, le
    redémarre s'il s'arrête et écrit `/var/log/dumbmonit-agent.log`.

    Fonctionne pareil sur TrueNAS CORE et d'autres appliances basées
    FreeBSD, où les séries ZFS sont l'objet de l'exercice.

=== "Windows (service)"

    ```powershell
    & ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_xxx -Url http://server:8080
    ```

    Le script télécharge `dumbmonit-agent-windows-x86_64.exe` depuis le
    serveur, le vérifie contre son SHA-256, l'installe comme
    `C:\Program Files\DumbMonit\dumbmonit-agent.exe`, écrit
    `C:\ProgramData\DumbMonit\agent.yaml` (lisible par SYSTEM et
    Administrators seulement) et enregistre le service Windows
    `DumbMonitAgent` (« DumbMonit system agent ») : démarré automatiquement
    au boot, en LocalSystem, et redémarré après un échec (10 s, 30 s, puis
    60 s). L'agent parle au gestionnaire de services lui-même
    (`--service`), arrêter le service le laisse donc envoyer ce qu'il a
    encore avant de sortir.

    Un service n'a pas de console : l'agent écrit son journal dans
    `C:\ProgramData\DumbMonit\agent.log`, mis de côté comme `agent.log.old`
    au-delà de 10 Mo.

    Il n'y a pas de build ARM64 natif : sur Windows 11 pour ARM, le script
    installe le binaire x86_64, qui tourne sous émulation x64.

Un jeton peut enrôler plusieurs machines : nommez-le d'après un groupe ou
une machine.

### Binaires natifs

L'agent est un seul exécutable statique par système, sans rien à
installer à côté :

| Système | Fichier | Où l'obtenir |
|---|---|---|
| Linux x86_64 | `dumbmonit-agent-linux-x86_64` | le serveur, `/download/…` |
| Linux aarch64 (ARM 64 bits : Raspberry Pi OS 64 bits, serveurs ARM) | `dumbmonit-agent-linux-aarch64` | le serveur, `/download/…` |
| Windows x86_64 | `dumbmonit-agent-windows-x86_64.exe` | le serveur, `/download/…` |
| FreeBSD x86_64 | `dumbmonit-agent-freebsd-x86_64` | le serveur, `/download/…` |
| macOS Apple silicon | `dumbmonit-agent-macos-aarch64` | la release GitHub (le serveur y redirige) |
| macOS Intel | `dumbmonit-agent-macos-x86_64` | la release GitHub (le serveur y redirige) |

Les binaires Linux sont liés statiquement contre musl : ils tournent sur
n'importe quelle distribution, glibc ou pas (Alpine, firmwares de NAS),
sans bibliothèque à faire correspondre. Chaque fichier a sa somme de
contrôle à côté, `<file>.sha256`, au format `sha256sum`. Chaque
[release GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
porte les six binaires et leurs sommes de contrôle, pour les machines qui
ne peuvent pas atteindre le `/download/` du serveur ou pour installer avec
`--bin`.

Pour construire vous-même les binaires Linux, Windows et FreeBSD, depuis
un clone du dépôt (Docker est le seul prérequis) :

```sh
docker buildx build --target agent-dist --output type=local,dest=dist/agent .
```

### Gérer le service

| | Linux (systemd) | Linux (OpenRC) | macOS (launchd) | FreeBSD (rc.d) | Windows (PowerShell) |
|---|---|---|---|---|---|
| Statut | `systemctl status dumbmonit-agent` | `rc-service dumbmonit-agent status` | `sudo launchctl print system/com.dumbmonit.agent` | `service dumbmonit_agent status` | `Get-Service DumbMonitAgent` |
| Logs | `journalctl -u dumbmonit-agent -f` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `tail -f /var/log/dumbmonit-agent.log` | `Get-Content C:\ProgramData\DumbMonit\agent.log -Tail 20 -Wait` |
| Redémarrer | `systemctl restart dumbmonit-agent` | `rc-service dumbmonit-agent restart` | `sudo launchctl kickstart -k system/com.dumbmonit.agent` | `service dumbmonit_agent restart` | `Restart-Service DumbMonitAgent` |
| Arrêter | `systemctl stop dumbmonit-agent` | `rc-service dumbmonit-agent stop` | `sudo launchctl bootout system/com.dumbmonit.agent` | `service dumbmonit_agent onestop` | `Stop-Service DumbMonitAgent` |
| Configuration | `/etc/dumbmonit/agent.yaml` | `/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `/usr/local/etc/dumbmonit/agent.yaml` | `C:\ProgramData\DumbMonit\agent.yaml` |
| Fichier de service | `/etc/systemd/system/dumbmonit-agent.service` | `/etc/init.d/dumbmonit-agent` | `/Library/LaunchDaemons/com.dumbmonit.agent.plist` | `/usr/local/etc/rc.d/dumbmonit_agent` | service `DumbMonitAgent` (`services.msc`) |

`--uninstall` (`-Uninstall` sur Windows) retire tout, service inclus, sur
chaque système.

!!! note "Le téléchargement est vérifié — toujours"
    La commande d'installation montrée par DumbMonit porte le SHA-256
    attendu de chaque binaire d'agent que le serveur fournit
    (`--sha256=linux-x86_64:…,linux-aarch64:…,freebsd-x86_64:…`,
    `-Sha256 windows-x86_64:…` sur Windows). Les deux installateurs
    vérifient le fichier téléchargé contre ça avant d'installer quoi que
    ce soit, et **refusent** d'installer quand ça ne correspond pas,
    quand aucune somme de contrôle n'est donnée pour la plateforme, ou
    quand aucun outil SHA-256 (`sha256sum`, `shasum`, `sha256`) n'est
    disponible.

    Les sommes de contrôle changent à chaque release du serveur : après
    une mise à jour de DumbMonit, copiez une commande d'installation
    fraîche depuis **Réglages → Agents** (ou depuis un nouveau jeton)
    plutôt que de relancer une ancienne, qui s'arrêterait avec « checksum
    mismatch ». Les mêmes sommes de contrôle sont montrées sous la
    commande d'installation, et publiées à `/download/<file>.sha256`.

    Pour les configurations isolées ou manuelles, `--insecure-skip-checksum`
    (`-InsecureSkipChecksum`) installe un binaire téléchargé sans le
    vérifier et le dit bruyamment ; `--bin=PATH` installe un fichier local
    tel quel, le fichier étant à vous de vérifier.

### Options de l'installateur

| `install.sh` | `install.ps1` | Effet |
|---|---|---|
| `--token=TOKEN` | `-Token` | Jeton d'enrôlement (requis). |
| `--url=URL` | `-Url` | URL du serveur, par exemple `http://server:8080`. Sur Linux, `DUMBMONIT_URL` dans l'environnement est utilisé si l'option est absente. |
| `--interval=N` | `-Interval` | Période d'échantillonnage en secondes (défaut 30). |
| `--services=a,b,c` | `-Services @('a','b')` | Services dont l'état est rapporté, nommés comme le système hôte les nomme : unités systemd sur Linux, labels launchd (`com.apple.sshd`) sur macOS, noms rc.d sur FreeBSD, services Windows. |
| `--tags=key=value,…` | `-Tags @{key='value'}` | Étiquettes, copiées comme `tag_<key>` sur chaque série. |
| `--hostname=NAME` | `-HostName` | Nom annoncé au serveur (défaut : celui de la machine). |
| `--bin=PATH` | `-BinPath` | Binaire local à installer plutôt que de le télécharger. |
| `--sha256=LIST` | `-Sha256` | SHA-256 attendu du téléchargement, comme des paires `platform-arch:hex` séparées par des virgules (une valeur hex seule fonctionne aussi). Inclus dans la commande que DumbMonit montre ; requis sauf si l'option suivante est donnée. |
| `--insecure-skip-checksum` | `-InsecureSkipChecksum` | Installer un binaire téléchargé sans le vérifier, avec un avertissement. Seulement pour les configurations isolées ou manuelles. |
| `--no-start` | | Tout installer, mais ne pas contacter le serveur ni démarrer le service (image de machine, test). |
| `--uninstall` | `-Uninstall` | Désinstaller l'agent et supprimer sa configuration. |
| `--help` | | Montrer l'aide. |

### Mise à jour {#upgrade}

Les deux scripts sont idempotents : relancer la commande d'installation,
avec le même jeton et la même URL, remplace le binaire, réécrit la
configuration et redémarre le service. C'est la procédure de mise à jour.
Prenez la commande depuis le serveur actuel (Réglages → Agents) : son
`--sha256` doit correspondre aux binaires de la version de serveur vers
laquelle vous avez mis à jour.

La même commande migre une machine qui fait encore tourner l'agent
EzyMonit, sur Linux et sur Windows. L'installateur arrête et retire
l'ancien service (`ezymonit-agent`, ou `EzyMonitAgent` sur Windows),
déplace la configuration vers le nouveau chemin
(`/etc/dumbmonit/agent.yaml`, `C:\ProgramData\DumbMonit\agent.yaml`) et
installe `dumbmonit-agent` à sa place. L'ancien jeton `ezym_…` reste
valide, la commande montrée à la création du jeton fonctionne donc
toujours ; rien n'a besoin d'être révoqué d'abord.

## Fichier de configuration {#configuration-file}

`/etc/dumbmonit/agent.yaml` sur Linux, `/usr/local/etc/dumbmonit/agent.yaml`
sur macOS et FreeBSD, `C:\ProgramData\DumbMonit\agent.yaml` sur Windows :

```yaml
server_url: http://server:8080
token: dmon_...
interval_secs: 30          # période d'échantillonnage
hostname: nas-basement     # optionnel : nom annoncé au serveur
services:                  # unités systemd ou services Windows
  - sshd
  - docker
tags:                      # étiquettes libres, préfixées "tag_" sur le serveur
  role: nas
  room: basement
docker: true               # inventaire de conteneurs (défaut : true)
docker_socket: /var/run/docker.sock
sensors: true              # températures et ventilateurs (défaut : true)
smart: true                # santé de disque via smartctl (défaut : true)
smart_bin: smartctl        # chemin, quand il n'est pas dans le PATH du service
smart_interval_secs: 300   # minimum 60 : un disque endormi n'est jamais réveillé pour être lu
zfs: true                  # pools ZFS via zpool (défaut : true)
zfs_bin: zpool
zfs_interval_secs: 120     # minimum 30
wireguard: true            # tunnels WireGuard via wg (défaut : true), voir ci-dessous
restic_repos: []           # dépôts restic à surveiller, voir "sauvegardes restic et Borg"
borg_repos: []             # dépôts Borg à surveiller
mdaemon: auto              # Windows : files MDaemon quand son service existe (true / false / auto)
hyperv: auto               # Windows : compteurs Hyper-V quand le service vmms existe (true / false / auto)
perf_counters:             # Windows : compteurs de performance à publier
  - '\Memory\Available MBytes'
  - path: '\Processor(_Total)\% Processor Time'
    name: cpu_total        # valeur du label "counter" ; le chemin quand omis
max_buffered_samples: 20000
log_level: info
```

`sensors`, `smart` et `zfs` sont détectés, jamais supposés : laissés à
`true` sur une machine sans sonde, sans `smartctl` ou sans pool, ils
coûtent un coup d'œil par période et n'émettent rien du tout. Réglez-en
un sur `false` pour arrêter même de regarder — ça vaut le coup sur un NAS
dont vous préférez que `smartctl` ne touche jamais les disques, bien que
l'agent passe déjà `-n standby` pour qu'un disque endormi ne soit mesuré
que quand il est réveillé pour d'autres raisons.

À côté, `/etc/dumbmonit/agent-secret` (`C:\ProgramData\DumbMonit\agent-secret`
sur Windows) contient le secret de liaison que le serveur a donné à cette
machine. L'agent l'écrit en mode `0600` et le relit à chaque démarrage ;
il ne fait pas partie d'`agent.yaml` parce que ce n'est pas quelque chose
que vous réglez — voir [Liaison](#binding-one-machine-one-agent).
`DUMBMONIT_AGENT_SECRET_FILE` le déplace, ce que vous voulez pour un agent
dans un conteneur.

Chaque clé peut être surchargée par l'environnement, ce qui rend l'agent
utilisable dans un conteneur sans monter un fichier :

| Variable | Effet |
|---|---|
| `DUMBMONIT_AGENT_CONFIG` | Chemin du fichier de configuration |
| `DUMBMONIT_AGENT_URL` | URL du serveur |
| `DUMBMONIT_AGENT_TOKEN` | Jeton d'enrôlement |
| `DUMBMONIT_AGENT_SECRET_FILE` | Où garder le secret de liaison (défaut : `agent-secret`, à côté du fichier de configuration) |
| `DUMBMONIT_AGENT_INTERVAL_SECS` | Période d'échantillonnage |
| `DUMBMONIT_AGENT_HOSTNAME` | Nom annoncé au serveur |
| `DUMBMONIT_AGENT_SERVICES` | Services à surveiller, séparés par des virgules |
| `DUMBMONIT_AGENT_TAGS` | `key=value`, séparés par des virgules |
| `DUMBMONIT_AGENT_DOCKER` | `true` / `false` |
| `DUMBMONIT_AGENT_DOCKER_SOCKET` | Chemin du socket Docker |
| `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | Conteneurs détaillés par hôte, défaut `200` (`0` : compteurs seulement) |
| `DUMBMONIT_AGENT_INTERFACES_IGNORE` | Interfaces laissées de côté, noms ou regex séparés par des virgules ; défaut `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` |
| `DUMBMONIT_AGENT_INTERFACES_ONLY` | Interfaces à garder ; quand réglé, remplace la liste d'exclusion |
| `DUMBMONIT_AGENT_MOUNTS_IGNORE` | Points de montage laissés de côté des systèmes de fichiers et de l'E/S disque ; par défaut ignore `/var/lib/docker/`, `/run/…`, `/sys/`, `/proc/`, `/dev/`, `/snap/` |
| `DUMBMONIT_AGENT_CPU_PER_CORE` | `true` pour aussi envoyer une série CPU par cœur (défaut `false`) |
| `DUMBMONIT_AGENT_SENSORS` | `true` / `false` : températures et ventilateurs (défaut `true`) |
| `DUMBMONIT_AGENT_SMART` | `true` / `false` : santé de disque via `smartctl` (défaut `true`) |
| `DUMBMONIT_AGENT_SMART_BIN` | Chemin de `smartctl` |
| `DUMBMONIT_AGENT_SMART_INTERVAL_SECS` | Période entre deux lectures SMART, défaut `300`, minimum `60` |
| `DUMBMONIT_AGENT_ZFS` | `true` / `false` : pools ZFS via `zpool` (défaut `true`) |
| `DUMBMONIT_AGENT_ZFS_BIN` | Chemin de `zpool` |
| `DUMBMONIT_AGENT_ZFS_INTERVAL_SECS` | Période entre deux lectures ZFS, défaut `120`, minimum `30` |
| `DUMBMONIT_AGENT_WIREGUARD` | `true` / `false` : tunnels WireGuard via `wg` (défaut `true`) |
| `DUMBMONIT_AGENT_WIREGUARD_BIN` | Chemin de `wg` |
| `DUMBMONIT_AGENT_RESTIC_BIN` | Chemin de `restic` |
| `DUMBMONIT_AGENT_BORG_BIN` | Chemin de `borg` |
| `DUMBMONIT_AGENT_BACKUP_REPOS_INTERVAL_SECS` | Période entre deux lectures des dépôts restic et Borg, défaut `600`, minimum `60` |
| `DUMBMONIT_AGENT_MDAEMON` | `true`, `false` ou `auto` : compteurs de performance de MDaemon (défaut `auto`, lus quand le service Windows `MDaemon` existe) |
| `DUMBMONIT_AGENT_HYPERV` | `true`, `false` ou `auto` : compteurs de performance d'Hyper-V (défaut `auto`, lus quand le service Windows `vmms` existe) |
| `DUMBMONIT_AGENT_PERF_COUNTERS` | Chemins de compteur de performance Windows, séparés par des **points-virgules** (une virgule peut faire partie d'un nom d'instance) ; remplace `perf_counters` |
| `DUMBMONIT_AGENT_MAX_BUFFERED_SAMPLES` | Taille du tampon de rattrapage |
| `DUMBMONIT_AGENT_LOG` | `trace`, `debug`, `info`, `warn`, `error` |
| `DUMBMONIT_AGENT_RELAY` | `true` pour faire tourner, pour le serveur, les sondes des appareils assignés à cet agent (voir [Faire tourner l'agent dans Docker / sur un autre réseau](#run-the-agent-in-docker--on-another-network)) |
| `DUMBMONIT_AGENT_SITE` | Libellé de site montré à côté de l'agent quand il est proposé comme relais |

## Compteurs de performance Windows {#windows-performance-counters}

Sur Windows, l'agent lit les compteurs de performance — les valeurs que
Performance Monitor (`perfmon`) montre — via la bibliothèque PDH, et les
publie à chaque période d'échantillonnage. Certains logiciels ne publient
leur état nulle part ailleurs.

**MDaemon.** Quand le service Windows nommé `MDaemon` existe, l'agent lit
l'objet de performance MDaemon : tailles de file, sessions, totaux de
message et de spam, l'état des serveurs internes de MDaemon et son
uptime. Les séries, les règles et la page qui les montre sont décrites sur
la page [MDaemon](mdaemon.md#mail-queues-through-the-agent).
`mdaemon: true` les lit même sans le service (un service renommé),
`mdaemon: false` ne le fait jamais. La présence du service est vérifiée à
nouveau toutes les dix périodes, un MDaemon installé après l'agent est
donc pris en compte sans redémarrage.

**Hyper-V.** Quand le service Hyper-V Virtual Machine Management
(`vmms`) existe, l'agent lit les objets de performance d'Hyper-V ; voir
[Hôtes Hyper-V](#hyper-v-hosts). `hyperv: true` et `hyperv: false` forcent
le choix, comme pour MDaemon.

**Tout autre compteur.** Listez son chemin complet sous `perf_counters`,
comme `\Object(Instance)\Counter` ou `\Object\Counter` pour un objet avec
une seule instance, et il est publié comme
`dumbmonit_agent_perf_counter{counter="…"}`, avec le nom que vous donnez
ou le chemin lui-même :

```yaml
perf_counters:
  - '\LogicalDisk(C:)\% Free Space'
  - path: '\MSExchangeTransport Queues(_total)\Messages Queued For Delivery'
    name: exchange_queue
```

- Écrivez les noms de compteur **en anglais**, comme ils apparaissent dans
  Performance Monitor sur un Windows anglais : l'agent les ajoute avec
  `PdhAddEnglishCounterW`, le même `agent.yaml` fonctionne donc sur un
  Windows français ou allemand.
- Mettez chaque chemin entre guillemets simples en YAML, pour que les
  barres obliques inverses et le `%` soient gardés tels quels.
- Un chemin malformé arrête l'agent au démarrage avec la clé et la
  raison. Un chemin que Windows ne connaît pas (le logiciel n'est pas
  installé, le nom est mal orthographié) est rapporté une fois dans le
  journal, `performance counter not available`, et les autres compteurs
  sont quand même lus ; il est retenté toutes les dix périodes.
- Les débits (`…/sec`) ont besoin de deux lectures : ils apparaissent à
  partir de la seconde période.
- 200 compteurs au plus sont lus. Sur Linux, macOS et FreeBSD le réglage
  est accepté et rien n'est lu.

Pour voir les noms exacts sur la machine, lancez `typeperf -q MDaemon`
(ou tout autre nom d'objet) dans une invite de commandes.

## Hôtes Hyper-V {#hyper-v-hosts}

Installé sur un hôte Hyper-V — Windows Server avec le rôle Hyper-V, ou
Windows avec Hyper-V activé — l'agent trouve le service `vmms` et lit les
propres compteurs de performance d'Hyper-V, sans rien à configurer. La
page de l'appareil de l'agent montre alors un panneau Hyper-V : combien
de machines virtuelles rapportent une santé saine, la vraie charge CPU de
l'hôte, la mémoire que chaque VM en cours détient, et le débit, la
latence et les erreurs de chaque disque virtuel.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Les noms de compteur viennent de la documentation de performance
    Hyper-V de Microsoft et ont été vérifiés contre la logique avec une
    source de compteur simulée, pas sur un hôte Hyper-V en fonctionnement.
    Un compteur que Windows ne connaît pas est rapporté une fois dans le
    journal de l'agent et les autres sont quand même lus. Dites-nous ce
    qui casse.

| Compteur | Série | Labels |
|---|---|---|
| `\Hyper-V Virtual Machine Health Summary\Health Ok`, `…\Health Critical` | `hyperv_vms_health_ok`, `hyperv_vms_health_critical` : machines virtuelles dans chaque état | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Total Run Time` | `hyperv_host_cpu_percent` : la charge de l'hôte, invités inclus | |
| `\Hyper-V Hypervisor Logical Processor(_Total)\% Guest Run Time` | `hyperv_host_cpu_guest_percent` : la part passée à faire tourner les invités | |
| `\Hyper-V Hypervisor\Partitions` | `hyperv_partitions` : VM en cours plus l'hôte lui-même | |
| `\Hyper-V Dynamic Memory VM(*)\Physical Memory`, `…\Guest Visible Physical Memory` | `hyperv_vm_physical_memory_megabytes`, `hyperv_vm_guest_visible_memory_megabytes` | `vm` |
| `\Hyper-V Dynamic Memory VM(*)\Current Pressure` | `hyperv_vm_memory_pressure_percent` : demande de mémoire sur mémoire assignée ; au-dessus de 100 la VM veut plus qu'elle n'a | `vm` |
| `\Hyper-V Virtual Storage Device(*)\Read Bytes/sec`, `…\Write Bytes/sec` | `hyperv_vhd_read_bytes_per_second`, `hyperv_vhd_write_bytes_per_second` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Latency`, `…\Queue Length` | `hyperv_vhd_latency_seconds` (moyenne), `hyperv_vhd_queue_length` | `disk` |
| `\Hyper-V Virtual Storage Device(*)\Error Count` | `hyperv_vhd_errors_total`, un compteur | `disk` |

Sur un hôte Hyper-V, `\Processor(_Total)\% Processor Time` ne mesure que
la propre partition de l'hôte, pas ce que les machines virtuelles
consomment : le compteur de processeur logique de l'hyperviseur est celui
qui dit si la machine est saturée. `vm` est le nom de la machine virtuelle
comme Hyper-V Manager le montre ; `disk` est le nom d'instance que
Windows donne à chaque disque virtuel, dérivé de son chemin de fichier.
Les instances `_Total` sont laissées de côté, et 200 VM ou disques au
plus sont lus par compteur.

Trois [règles intégrées](../alerting/rules.md#hyper-v) s'appliquent :

- **Santé de VM Hyper-V critique** : Hyper-V rapporte une VM en santé
  critique depuis cinq minutes (Avertissement) — le plus souvent un
  disque qui a manqué d'espace.
- **CPU d'hôte Hyper-V élevé** : les processeurs logiques occupés à plus
  de 90 % depuis quinze minutes (Avis).
- **Erreurs de disque virtuel Hyper-V** : un disque virtuel a rapporté
  des erreurs d'E/S dans la dernière heure (Avis).

## Faire tourner l'agent dans Docker / sur un autre réseau {#run-the-agent-in-docker--on-another-network}

L'agent est aussi publié comme image conteneur, avec les mêmes tags que
le serveur : `ghcr.io/laupernoe/dumbmonit-agent:latest` (dernière
release), `:edge` (dernier commit sur `main`). C'est une image
`FROM scratch` contenant le binaire d'agent et le paquet de CA Mozilla,
configurée entièrement par variables d'environnement. Le dépôt fournit un
`docker-compose.agent.yml` prêt à l'emploi :

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=docker-host-1 \
docker compose -f docker-compose.agent.yml up -d
```

Ce que le conteneur voit, et quoi monter :

| Besoin | Quoi faire | Pourquoi |
|---|---|---|
| CPU, mémoire, charge, E/S disque de l'hôte | Rien | `/proc/stat`, `/proc/meminfo`, `/proc/loadavg` et `/proc/diskstats` sont à l'échelle de l'hôte dans tout conteneur. |
| Nom d'hôte | `DUMBMONIT_AGENT_HOSTNAME` | Le nom d'hôte d'un conteneur est un id aléatoire ; l'appareil serait nommé d'après lui. |
| Conteneurs de l'hôte | `- /var/run/docker.sock:/var/run/docker.sock:ro` | Optionnel. Listes et inspections en lecture seule ; les actions Redémarrer/Mettre à jour demandent qu'il soit accessible en écriture et `DUMBMONIT_AGENT_COMMANDS=true`. Sans le socket, l'agent rapporte simplement aucun conteneur. |
| Usage du système de fichiers | `- /:/host:ro` (ou les montages qui vous intéressent) | L'agent rapporte les montages visibles *à l'intérieur* du conteneur, sous leur chemin de conteneur. |
| Compteurs réseau de l'hôte | `network_mode: host` | Les interfaces sont namespacées : sans ça l'agent voit un `veth`. |
| Moniteurs ping relayés via cet agent | `cap_add: [NET_RAW]` | ICMP a besoin d'une socket brute. |
| CA privée | `- ./ca-bundle.crt:/etc/ssl/certs/ca-certificates.crt:ro` | Serveur derrière un reverse proxy interne, appareils avec des certificats autosignés. |
| Garder la liaison à travers les recréations | `- agent-state:/var/lib/dumbmonit-agent` et `DUMBMONIT_AGENT_SECRET_FILE=/var/lib/dumbmonit-agent/agent-secret` | Le secret de liaison vit dans le système de fichiers du conteneur, qu'un `docker compose up --force-recreate` jette. Sans le volume, l'agent revient non lié et quelqu'un doit autoriser le réenrôlement depuis la page de l'appareil chaque fois. |

`pid: host` et `privileged: true` ne sont jamais nécessaires : l'agent ne
collecte aucune métrique par processus.

### Mode relais : surveiller un autre site

Un agent peut faire plus que surveiller sa propre machine. Avec
`relay: true` (ou `DUMBMONIT_AGENT_RELAY=true`), le serveur **délègue des
sondes** à lui : les appareils dont le réglage *Atteint via* nomme cet
agent ne sont pas sondés par le serveur mais par l'agent, depuis son
propre réseau, avec exactement les mêmes collecteurs — SNMP, Proxmox VE,
Proxmox Backup Server, Synology, HTTP, TCP, DNS, ping, TLS. Les résultats
atterrissent sous l'appareil comme si le serveur l'avait sondé : mêmes
métriques, même *dernière sonde*, mêmes alertes. Un bureau distant, un
second homelab ou le réseau d'un client se montrent ainsi dans le seul
DumbMonit que vous avez déjà, sans VPN et sans rien à ouvrir du côté
distant.

Comment le configurer, étape par étape, est dans
[Surveiller un site distant](../install/remote-site.md).

## Liaison : une machine, un agent {#binding-one-machine-one-agent}

Un jeton d'enrôlement dit *cette machine est autorisée à parler*. Il ne
dit pas *laquelle* parle — c'est à ça que sert la clé d'identité
(`/etc/machine-id`, ou le nom d'hôte), et cette clé n'est pas un secret :
elle peut être lue sur n'importe quelle machine en quelques secondes.

Donc au premier lot le serveur donne à chaque machine un **secret de
liaison** qui lui est propre. Il n'en garde qu'une empreinte ; l'agent
écrit le secret dans `/etc/dumbmonit/agent-secret` en mode `0600` et le
présente dans l'en-tête `X-DumbMonit-Agent-Secret` à chaque requête
ensuite — mesures, commandes de conteneur et sondes relayées pareil. À
partir de là :

- aucune autre machine ne peut envoyer de mesures sous cet appareil, ni
  réécrire son nom d'hôte et son système d'exploitation ;
- aucune autre machine ne peut récupérer — et donc consommer — les
  commandes Redémarrer et Mettre à jour mises en file pour elle.

La page de l'appareil montre où en est une machine, sous
**Agent → Liaison** :

| État | Ce que ça signifie | Quoi faire |
|---|---|---|
| **Liée** | L'agent détient un secret qui lui est propre. | Rien. |
| **Non liée — réenrôler cet hôte** | Un hôte enrôlé avant que la liaison n'existe, et jamais lié depuis. Le serveur refuse ses mesures, commandes de conteneur et sondes relayées. | Cliquez sur **Autoriser le réenrôlement** sur la page de l'appareil, puis redémarrez l'agent. Si l'agent est antérieur à la liaison, relancez d'abord la commande d'installation sur l'hôte. |

### Hôtes antérieurs à la liaison {#hosts-that-predate-binding}

!!! warning "Changé : les hôtes non liés sont refusés"
    Les releases antérieures continuaient d'accepter les hôtes enrôlés
    avant que la liaison n'existe, tant qu'ils utilisaient le jeton qui
    les avait enrôlés, et les liaient silencieusement une fois leur agent
    mis à jour. Cette fenêtre de transition est fermée : un hôte qui ne
    s'est jamais lié est maintenant **refusé sur chaque route d'agent** —
    mesures, commandes de conteneur et sondes relayées — jusqu'à ce qu'un
    administrateur le réenrôle. Un jeton de parc est partagé par
    plusieurs machines et la clé d'identité peut être devinée, rien
    d'autre ne prouve donc que l'agent qui demande est bien celui de
    l'hôte.

    Un binaire d'agent trop ancien pour recevoir un secret de liaison est
    aussi refusé, y compris à son premier enrôlement.

Pour ramener un tel hôte :

1. Si son agent est antérieur à la liaison, relancez la commande
   d'installation sur lui (le binaire d'agent doit pouvoir stocker un
   secret de liaison).
2. Ouvrez sa page d'appareil et cliquez sur **Agent → Autoriser le
   réenrôlement**. Ça ouvre une fenêtre d'une heure.
3. Redémarrez l'agent, ou attendez sa prochaine tentative : ce lot lie
   l'hôte avec un secret frais et ferme la fenêtre. Un jeton autre que
   celui qui a enrôlé l'hôte doit toujours pouvoir enrôler (pas épuisé,
   pas expiré).

Jusque là, le journal de l'agent porte l'explication du serveur :

```
WARN this agent is not recognised for this machine
     reason: This machine was enrolled before agent binding and was never bound
     to its agent installation, so the server no longer accepts it. Re-enrol
     this host: open its device page in DumbMonit, click 'Allow re-enrolment',
     then restart the agent (re-run the install command first if the agent is
     older than binding).
```

### Réenrôlement après une réinstallation {#re-enrolment-after-a-reinstall}

Si la machine est reconstruite, son disque remplacé, ou son conteneur
d'agent recréé sans le volume qui détient le secret, l'agent revient sans
lui et le serveur refuse ses lots. Ce refus est délibéré : une machine
qui est liée reste liée jusqu'à ce qu'un humain dise le contraire, sinon
la liaison ne protégerait rien.

Le chemin de retour est sur la page de l'appareil : **Agent → Autoriser le
réenrôlement**. Ça ouvre une fenêtre d'une heure pendant laquelle un agent
présentant un jeton d'enrôlement valide lie à nouveau cette machine, avec
un secret frais ; l'ancien secret arrête de fonctionner, et la fenêtre se
ferme dès qu'elle est utilisée. L'agent retente de lui-même pendant ce
temps, il n'y a donc rien à redémarrer sur la machine — et rien n'est
perdu, puisque ses lectures restent en tampon jusqu'à ce que le lien soit
accepté à nouveau.

Jusqu'à ce que quelqu'un ouvre cette fenêtre, le journal de l'agent dit
quoi faire :

```
WARN this agent is not recognised for this machine
     reason: This machine is already enrolled and bound to another agent
     installation. If you reinstalled it, open its device page in DumbMonit and
     click 'Allow re-enrolment', then restart the agent.
```

## Jetons d'enrôlement

- Les jetons sont créés dans **Réglages → Agents** ou en ajoutant un
  appareil de type agent. Le serveur ne garde qu'une empreinte : le jeton
  en clair est montré une fois, dans la réponse de création, avec les
  deux commandes d'installation.
- L'agent l'envoie comme `Authorization: Bearer dmon_…` sur
  `POST /api/ingest`. Il n'apparaît jamais dans les journaux de l'agent,
  même tronqué.
- **Réglages → Agents** liste les jetons avec leur préfixe, leur portée,
  leur date de création et leur dernière utilisation. **Révoquer** arrête
  chaque agent qui utilise ce jeton à son prochain envoi ; les machines
  deviennent alors *inaccessibles* après trois périodes manquées.
  Relancez l'installateur avec un nouveau jeton pour les réenrôler.
- Les machines sont des appareils séparés : révoquer un jeton ne les
  supprime pas.

### Usage unique ou parc {#single-use-or-fleet}

La portée d'un jeton est choisie à sa création, et c'est un choix, pas un
défaut qui vous arrive :

| Portée | Ce qu'il enrôle | Quand |
|---|---|---|
| **Usage unique** (le défaut) | Exactement une machine. | Une installation. Une commande d'installation vit dans l'historique du shell, dans un ticket, dans un log de chat ; une à usage unique ne vaut rien une fois utilisée. |
| **Réutilisable pour un parc** | Autant de machines que vous autorisez — un nombre, ou pas de limite. | Un playbook, une image de machine, un lot d'installations. |

Les deux peuvent porter une échéance en jours. Une échéance n'arrête que
les *enrôlements* : les machines déjà enrôlées continuent de rapporter
après qu'elle passe, ce que vous voulez pour une fenêtre d'installation.
Révoquer est ce qui coupe un parc.

Un jeton qui ne peut plus enrôler — épuisé, ou passé son échéance — se
montre comme **N'enrôle plus** dans Réglages → Agents. Ses machines
continuent de rapporter ; il ne peut simplement plus en laisser entrer une
nouvelle.

## Pannes et diagnostics

Quand le serveur est inaccessible, l'agent continue de mesurer et met ses
lectures en tampon mémoire avec leurs horodatages d'origine. Elles sont
envoyées dès que le lien revient : un redémarrage de serveur ne laisse
aucun trou dans les graphiques. Le tampon est borné (`max_buffered_samples`,
environ une heure par défaut) et abandonne les lectures les plus
anciennes quand il est plein. Les tentatives reculent de 5 secondes à
5 minutes avec du jitter.

```sh
dumbmonit-agent --dry-run          # affiche les lectures, n'envoie rien
dumbmonit-agent --once             # envoie un lot puis sort
journalctl -u dumbmonit-agent -f   # journal du service (systemd)
tail -f /var/log/dumbmonit-agent.log   # journal du service (OpenRC)
```

## Erreurs courantes

| Symptôme | Cause probable |
|---|---|
| L'installateur dit « could not reach the server » | Mauvaise URL ou jeton. La configuration est écrite : corrigez `agent.yaml`, puis redémarrez le service. |
| Le téléchargement échoue avec 404 | Le serveur n'a pas de binaires d'agent (`DUMBMONIT_AGENT_DIR` vide). Utilisez `--bin=PATH` avec un binaire que vous avez construit. |
| L'installateur s'arrête avec « checksum mismatch » | La commande a été copiée avant une mise à jour du serveur (copiez-en une fraîche depuis Réglages → Agents), ou le binaire reçu n'est pas celui que le serveur sert : un proxy ou un cache sur le chemin, ou un téléchargement altéré. |
| L'installateur s'arrête avec « no expected checksum » | La commande n'a pas de `--sha256` pour cette plateforme : une ancienne commande, ou un binaire macOS (pas fourni par le serveur). Copiez une commande fraîche, ou passez `--sha256=<platform>-<arch>:<hex>` depuis le `.sha256` de la release. |
| La machine n'apparaît jamais | L'envoi va vers `/api/ingest` sur l'URL de la commande d'installation ; derrière un reverse proxy, assurez-vous qu'il est transféré et que les corps jusqu'à 16 Mo sont autorisés. |
| *Inaccessible* bien que l'agent tourne | Le jeton a été révoqué, ou les envois sont rejetés. Vérifiez `journalctl -u dumbmonit-agent` : l'erreur y est journalisée. |
| Le journal dit « this agent is not recognised for this machine » | La machine est liée à une installation d'agent dont celui-ci n'a pas le secret — une réinstallation, ou un conteneur recréé sans son volume d'état. Ouvrez la page de l'appareil et cliquez sur **Autoriser le réenrôlement** ; voir [Réenrôlement après une réinstallation](#re-enrolment-after-a-reinstall). |
| Le journal dit « this machine was enrolled before agent binding » | Un hôte qui ne s'est jamais lié à son agent. Cliquez sur **Autoriser le réenrôlement** sur sa page d'appareil, puis redémarrez l'agent ; voir [Hôtes antérieurs à la liaison](#hosts-that-predate-binding). |
| Le journal dit « this agent is too old to be bound to its machine » | Le binaire d'agent ne peut pas stocker de secret de liaison. Relancez la commande d'installation sur l'hôte ; si l'appareil existe déjà, cliquez d'abord sur **Autoriser le réenrôlement**. |
| Le journal dit « this enrollment token has already enrolled all the machines it was allowed to » | Un jeton à usage unique réutilisé. Créez-en un nouveau, ou un jeton réutilisable dans Réglages → Agents. |
| Windows | Les parties du service Windows n'avaient pas été exercées dans l'image de build du projet au moment de l'écriture ; rapportez les problèmes sur GitHub. |

## Conteneurs Docker {#docker-containers}

Quand l'agent peut lire le socket Docker (`docker: true`, le défaut, et
un utilisateur dans le groupe `docker` ou root —
`usermod -aG docker dumbmonit`, puis redémarrez l'agent), chaque
conteneur devient une poignée de séries, étiquetées `container` et
`image` (le `name:tag` que montre `docker ps`). Si la page de l'appareil
dit « No Docker on this machine » tandis que Docker est installé, c'est
la première chose à vérifier.

| Métrique | Signification |
|---|---|
| `container_up` | 1 en cours, 0 sinon |
| `container_health` | 0 pas de healthcheck, 1 sain, 2 malsain, 3 démarrage |
| `container_restart_count` | Le compteur de redémarrage de Docker pour le conteneur |
| `container_started_seconds` | Secondes depuis le dernier démarrage du conteneur (0 quand arrêté) |
| `container_image_age_seconds` | Secondes depuis la construction de l'image |
| `container_update_available` | 1 quand le registre a une image plus récente pour le même tag, 0 quand à jour, -1 inconnu |

La vérification de mise à jour compare le digest de l'image locale avec
celui que le registre annonce pour le tag
(`HEAD /v2/<name>/manifests/<tag>`, flux de jeton anonyme sur Docker Hub
et GHCR). Elle tourne en arrière-plan au plus une fois par heure par
image et ne retarde jamais le cycle de 30 secondes ; un registre privé qui
refuse l'accès anonyme se lit simplement comme -1. Désactivez-la avec
`docker_update_check: false` (`DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK`).

Règles intégrées : **Conteneur arrêté** (`container_up == 0` depuis 2
minutes), **Conteneur malsain** (`container_health == 2` depuis 3
minutes), **Conteneur redémarre** (trois redémarrages ou plus en 15
minutes), **Mise à jour de conteneur disponible** (info, après une heure,
répétée chaque jour). L'alerte nomme le conteneur.

### Actions

Sur la page de l'appareil, chaque conteneur a deux actions et deux
interrupteurs. Les actions voyagent via un **canal de commande** : le
serveur met la commande en file, l'agent la récupère après son prochain
lot (`GET /api/agent/commands`, même jeton d'enrôlement), la lance, et
rapporte le résultat (`queued → running → done | failed`), avec un court
journal que vous pouvez déplier dans l'interface. Les commandes plus
vieilles que dix minutes ne sont jamais exécutées ; l'agent lance une
commande à la fois.

Une commande en file peut être **annulée** depuis l'interface tant que
l'agent ne l'a pas récupérée. Une commande que personne n'est venu
chercher **expire** après dix minutes, sur le serveur, que l'agent la
sonde ou pas — une machine dont l'agent est arrêté ne garde donc jamais
un « En file » périmé pour toujours, et ne bloque jamais la prochaine
action sur ce conteneur.

L'agent dit au serveur, à chaque lot, s'il accepte les commandes. Quand
il ne le fait pas — un agent plus ancien que le canal de commande, ou
installé avec `commands: false` — la page de l'appareil le dit plutôt que
de montrer Redémarrer et Mettre à jour, l'API refuse les commandes
(`409`), et les politiques ne mettent rien en file. Réinstallez l'agent
avec l'installateur actuel pour activer les actions.

- **Redémarrer** — `docker restart` pour un conteneur en cours,
  `docker start` pour un arrêté, puis une vérification de 20 secondes
  qu'il tourne.
- **Mettre à jour maintenant** — récupère le tag que le conteneur utilise
  déjà, crée un nouveau conteneur avec la même configuration
  (environnement, montages, ports, labels, réseaux, politique de
  redémarrage), arrête l'ancien, échange les noms, démarre le nouveau et
  attend son healthcheck (ou 20 secondes plus une vérification de
  fonctionnement quand l'image n'en a pas). Si quoi que ce soit échoue,
  l'agent **revient en arrière** : le nouveau conteneur est retiré et
  l'ancien redémarré, et la commande rapporte `failed` avec la raison. En
  cas de succès, l'ancien conteneur est retiré et, quand « prune » est
  activé, l'ancienne image aussi — sauf si un autre conteneur l'utilise
  encore.

Les conteneurs gérés par Compose (labels `com.docker.compose.*`) sont
mis à jour quand même, mais le résultat avertit que le prochain
`docker compose up` recréera le conteneur depuis le fichier compose. Les
conteneurs dont le nom commence par `dumbmonit` ou `dumbmonit` sont
refusés : le moniteur ne se met jamais lui-même à jour.

### Politiques

Une **politique** par conteneur est évaluée par le serveur chaque
minute :

- **Redémarrer si arrêté** — un conteneur rapportant `container_up == 0`
  est redémarré automatiquement, au plus une fois toutes les dix minutes.
- **Mise à jour automatique dans les fenêtres de maintenance** — un
  conteneur avec une mise à jour disponible est mis à jour
  automatiquement, mais seulement tandis qu'une fenêtre de maintenance
  (silence) couvre l'appareil, au plus une fois toutes les six heures.
  Planifiez-en une sur la page Alertes, ou mettez l'appareil en silence
  depuis sa page. Les politiques pruning l'ancienne image et les fenêtres
  de maintenance seulement par défaut.

Deux endroits les règlent :

- la **bande Docker** sous l'en-tête de l'appareil : « 28 conteneurs · 27
  en cours · 3 mises à jour disponibles · politiques : 2 auto-redémarrage,
  0 auto-mise à jour », la dernière action et son statut, **Gérer les
  conteneurs** (ouvre la liste ci-dessous) et **Politiques…**, un tableau
  de chaque conteneur avec les deux interrupteurs et une ligne « Appliquer
  à tous » — la façon la plus rapide d'activer l'auto-redémarrage pour
  toute une machine ;
- les interrupteurs à l'intérieur de chaque ligne de la section
  **Conteneurs**, à côté des boutons Redémarrer et Mettre à jour
  maintenant.

Chaque interrupteur s'enregistre seul
(`PUT /api/targets/<id>/containers/<name>/policy`) ; un échec ne concerne
que cette ligne et est montré à côté.

Les actions automatiques se montrent dans « Actions récentes » comme
demandées par `policy`.

### Règles de sécurité

- L'agent n'agit que quand `commands: true` (le défaut) ; réglez-le sur
  `false` pour une machine qui ne doit être que mesurée.
- L'agent récupère les commandes lui-même, avec son jeton d'enrôlement —
  rien ne se connecte à la machine. Elles expirent après dix minutes et
  tournent une à la fois ; l'agent ne récupère jamais un tag différent de
  celui que le conteneur utilise déjà.
- Une mise à jour qui ne revient pas saine est annulée vers le conteneur
  précédent, et l'ancienne image n'est retirée qu'une fois que le nouveau
  conteneur tourne.
- Les conteneurs nommés `dumbmonit*` ou `ezymonit*` sont refusés, à la
  main ou par politique : le moniteur n'agit jamais sur ses propres
  conteneurs. La vérification est faite deux fois — le serveur refuse de
  mettre la commande en file, et l'agent refuse de la lancer — et l'agent
  la refait sur le nom que Docker rapporte après inspection du conteneur,
  nommer l'un par son id ne change donc rien. L'agent refuse aussi d'agir
  sur son propre conteneur, quel que soit son nom : il ne serait pas là
  pour finir le travail ou le rapporter.
- Les conteneurs gérés par Compose sont mis à jour, mais le prochain
  `docker compose up` les recrée depuis le fichier compose : montez le
  tag là aussi.

## Sauvegardes Plakar

L'agent peut surveiller les klosets (dépôts) [Plakar](https://plakar.io) :
toutes les dix minutes, en arrière-plan, il lance `plakar at <kloset> ls`
et `info` sur chacun et réémet la dernière lecture à chaque cycle.

Plakar est **détecté, jamais supposé**. Au démarrage et puis à chaque
cycle jusqu'à ce qu'il trouve quelque chose, l'agent cherche le binaire
`plakar` (sur `PATH`, ou `plakar_bin`) et des klosets dans les
emplacements standards. Quand il n'y a aucun binaire, aucun kloset et
aucune trace de Plakar (`~/.cache/plakar`), l'agent n'émet aucune série
`backup_*` du tout, le dit une fois au niveau `debug`, et la page de
l'appareil ne montre aucun panneau Backups — rien à configurer sur les
machines sans Plakar. Pour sauter la détection entièrement :

```yaml
plakar: false                        # ou DUMBMONIT_AGENT_PLAKAR=false
```

### Découverte

Rien à configurer : quand `plakar_klosets` est vide, l'agent trouve les
klosets lui-même, dans cet ordre, et lit chaque emplacement une seule
fois :

1. le propre répertoire de configuration de l'agent —
   `$XDG_CONFIG_HOME/plakar`, ou `~/.config/plakar` du compte sous lequel
   il tourne (`plakar_home` quand réglé) ;
2. `/root/.config/plakar` ;
3. `/home/*/.config/plakar`, par ordre alphabétique ;
4. `/var/lib/plakar`, quand ce répertoire existe.

Dans chaque répertoire il lit `stores.yml` (ou `stores.yaml` ; l'ancien
`plakar.yml` est accepté aussi) et prend chaque entrée de la carte
`stores` (aussi `repositories` et `klosets`) qui a une `location`. Seule
la location est lue : les phrases de passe et secrets rclone qui se
trouvent à côté ne sont jamais gardés ni journalisés. Chaque entrée
devient le kloset `@<name>`, lu avec `plakar -configdir <dir> at @<name>`,
un kloset créé par un utilisateur est donc lu même quand l'agent tourne
en root. `~/.plakar` de chaque compte est ajouté quand il existe (le
kloset par défaut de `plakar create`). Deux comptes nommant des klosets
différents de la même façon sont distingués comme `@name` et
`<user>:@name`.

Le résultat est journalisé une fois à `info` (et à nouveau seulement
quand ça change). Deux jauges, émises seulement une fois Plakar détecté,
laissent l'interface distinguer les cas restants :
`backup_plakar_present` (1 quand le binaire tourne) et
`backup_klosets_found` (nombre de klosets lus). La page de l'appareil dit
alors « Plakar est installé mais aucun kloset n'a été trouvé », ou liste
les klosets. Quand des klosets existent mais que le binaire ne tourne
pas (un `PATH` de service sans `plakar`, typiquement), ils sont listés
comme illisibles avec une indication de régler `plakar_bin`, et l'agent
avertit une fois.

Pour surveiller une liste fixe à la place, nommez-la — la découverte est
alors sautée :

```yaml
plakar: true                         # défaut : détecter Plakar ; false le cache entièrement
plakar_bin: /usr/bin/plakar          # défaut : "plakar" sur PATH
plakar_klosets:                      # liste explicite : remplace la découverte
  - /srv/backups/main
  - "@diskext"                       # un store nommé du compte de l'agent
plakar_home: /home/noe               # optionnel : HOME (et répertoire de config) utilisé pour lancer plakar
plakar_interval_secs: 600            # minimum 60
```

| Variable | Effet |
|---|---|
| `DUMBMONIT_AGENT_PLAKAR` | `true` / `false` — surveiller les sauvegardes Plakar quand détecté (défaut `true`) |
| `DUMBMONIT_AGENT_PLAKAR_BIN` | Chemin du binaire `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_KLOSETS` | Klosets, séparés par des virgules (remplace la découverte) |
| `DUMBMONIT_AGENT_PLAKAR_HOME` | Répertoire home utilisé pour lancer `plakar` |
| `DUMBMONIT_AGENT_PLAKAR_INTERVAL_SECS` | Secondes entre deux lectures |
| `DUMBMONIT_AGENT_DOCKER_UPDATE_CHECK` | `true` / `false` — vérification de registre pour les mises à jour de conteneur |
| `DUMBMONIT_AGENT_COMMANDS` | `true` / `false` — accepter les actions du serveur |

Un kloset chiffré a besoin de sa phrase de passe : mettez-la dans
l'entrée du store (`plakar store set <name> passphrase=…`, ce que
`plakar store add` fait) ou exportez `PLAKAR_PASSPHRASE` dans
l'environnement du service de l'agent. Sans ça, le kloset se lit comme
« Ne peut pas être ouvert » (`backup_last_status = 0`).

### Métriques et règles

Métriques, étiquetées `kloset` et `source` (le répertoire qui a été
sauvegardé) : `backup_last_success_seconds` (âge du snapshot le plus
récent), `backup_snapshot_count`, `backup_last_status` (1 quand le
snapshot le plus récent n'a pas d'erreur, 0 quand il en a ou que le
kloset ne peut pas être ouvert), `backup_size_bytes{kloset}` (taille de
stockage du dépôt), plus `backup_plakar_present` et `backup_klosets_found`
décrits ci-dessus. Aucune d'elles n'existe tant que Plakar n'est pas
détecté, les règles intégrées ci-dessous ne peuvent donc pas se
déclencher sur une machine sans Plakar.

Règles intégrées : **Sauvegarde Plakar trop ancienne** (avis, aucun
snapshot depuis plus de deux jours) et **Sauvegarde Plakar échouée**
(avertissement). La page de l'appareil montre un bloc par kloset — âge
de la dernière sauvegarde, nombre de snapshots, taille, dernier statut —
et une ligne par source, avec une indication « Sauvegarde trop ancienne »
au-delà de deux jours.

### Lancer une première sauvegarde

Avec Plakar 1.1 installé sur la machine (`plakar version`), comme
l'utilisateur qui possédera les sauvegardes :

```sh
# 1. un store de kloset, nommé pour que l'agent le trouve dans ~/.config/plakar/stores.yml
plakar store add diskext /mnt/diskext/plakar passphrase=…   # ou tout emplacement que Plakar accepte
plakar at @diskext create                                   # ajoutez -plaintext pour éviter le chiffrement

# 2. une première sauvegarde, puis vérifiez-la
plakar at @diskext backup /srv/photos
plakar at @diskext ls
```

`plakar at <path> create` et `plakar at <path> backup <dir>` fonctionnent
pareil avec un simple répertoire plutôt que `@name` ; `plakar create` sans
`at` utilise `~/.plakar`, que l'agent trouve aussi. Planifiez
`plakar at @diskext backup /srv/photos` avec cron ou un timer systemd :
l'agent rapporte l'âge du snapshot le plus récent, et alerte quand un
jour passe sans aucun — deux jours pour la règle intégrée.

Pour l'essayer localement avec un kloset non chiffré :

```sh
mkdir -p /tmp/plakar-demo-src && echo hello > /tmp/plakar-demo-src/a.txt
plakar at /tmp/plakar-demo create -plaintext
plakar at /tmp/plakar-demo backup /tmp/plakar-demo-src
plakar at /tmp/plakar-demo ls
```

puis ajoutez `plakar_klosets: ["/tmp/plakar-demo"]` à `agent.yaml` (un
chemin nu hors d'un home n'est pas découvert) et vérifiez avec
`dumbmonit-agent --dry-run` que les échantillons `backup_*` apparaissent.

## Sauvegardes restic et Borg {#restic-and-borg-backups}

L'agent peut surveiller des dépôts [restic](https://restic.net) et
[Borg](https://www.borgbackup.org) : toutes les dix minutes, en
arrière-plan, il ouvre chaque dépôt que vous déclarez et rapporte s'il a
pu être lu et l'âge de son snapshot le plus récent. Rien n'est découvert :
un dépôt n'a pas d'emplacement standard et ne peut pas être ouvert sans
son mot de passe, chacun est donc listé dans `agent.yaml`, avec la façon
d'obtenir son mot de passe.

**Le mot de passe ne quitte jamais la machine.** L'agent le donne à
`restic` ou `borg` seulement via l'environnement du processus enfant ; il
n'est jamais journalisé, jamais envoyé au serveur, et les séries ne
portent que le nom du dépôt.

```yaml
restic_repos:
  - name: nas                                   # le label "repo" ; défaut : le dépôt, sans identifiants
    repository: sftp:backup@nas.lan:/srv/restic  # passé comme RESTIC_REPOSITORY, jamais sur la ligne de commande
    password_file: /etc/dumbmonit/restic-nas.pass
    host: web-01                                # optionnel : seulement les snapshots de cet hôte (--host), pour un dépôt partagé
    args: ["-o", "sftp.command=ssh -i /etc/dumbmonit/id_backup -o UserKnownHostsFile=/etc/dumbmonit/known_hosts backup@nas.lan -s sftp"]
  - name: b2
    repository: s3:https://s3.eu-central-003.backblazeb2.com/my-bucket
    password_command: cat /etc/dumbmonit/restic-b2.pass   # ou toute commande qui l'affiche (RESTIC_PASSWORD_COMMAND)
    env:                                        # environnement supplémentaire pour ce dépôt seulement
      AWS_ACCESS_KEY_ID: "…"
      AWS_SECRET_ACCESS_KEY: "…"
borg_repos:
  - name: offsite
    repository: ssh://borg@offsite.example.net/./backups   # passé comme BORG_REPO
    password_file: /etc/dumbmonit/borg-offsite.pass        # lu par l'agent, passé comme BORG_PASSPHRASE
    glob_archives: "web-01-*"                   # optionnel : seulement ces archives (--glob-archives)
    env:
      BORG_RSH: "ssh -i /etc/dumbmonit/id_borg -o UserKnownHostsFile=/etc/dumbmonit/known_hosts"
restic_bin: restic                              # défaut : sur PATH
borg_bin: borg
backup_repos_interval_secs: 600                 # minimum 60
```

Gardez `agent.yaml` et les fichiers de mot de passe lisibles par root
seulement (`chmod 600`), comme l'installateur le fait pour `agent.yaml`.
`password_command` correspond à `RESTIC_PASSWORD_COMMAND` ou
`BORG_PASSCOMMAND`. Tout ce que les outils lisent depuis leur environnement
peut aller sous `env` (`RESTIC_PASSWORD`, `BORG_PASSPHRASE`, clés cloud,
`BORG_RSH`…), et le propre environnement de service de l'agent est aussi
transmis.

Ce que l'agent lance, en lecture seule dans le sens le plus strict :

- `restic snapshots --json --latest 1 --no-lock --no-cache` : aucun
  verrou n'est écrit, un dépôt en lecture seule ou append-only peut donc
  être surveillé, et un `prune` tenant le verrou exclusif ne fait pas
  paraître le dépôt en panne ; pas de cache non plus, puisque le service
  installé ne peut pas écrire dans `/root`.
- `borg --bypass-lock list --json --last 1` : sans `--bypass-lock`, la
  lecture échouerait pendant toute la durée de chaque `borg create`, qui
  tient le verrou exclusif. Borg écrit un petit enregistrement de
  sécurité sous `~/.config/borg` ; quand ce home n'est pas accessible en
  écriture (le service installé tourne avec `ProtectHome=yes`), l'agent
  pointe `BORG_BASE_DIR` vers son `/tmp` privé à la place, sauf si vous
  réglez `BORG_BASE_DIR` vous-même. Un dépôt en mode `keyfile` a alors
  besoin de `BORG_KEY_FILE` dans son `env`.

Comme le service tourne avec `ProtectHome=yes`, gardez ce dont les outils
ont besoin hors des répertoires home : fichiers de mot de passe, clés SSH
et `known_hosts` sous `/etc/dumbmonit/`, comme dans l'exemple, et les
dépôts locaux sous `/srv`, `/mnt` ou `/var` plutôt que `/home`. Les
lectures tournent l'une après l'autre, dans la limite de mémoire du
service (128 Mio avec l'agent lui-même).

Métriques, étiquetées `tool` (`restic` ou `borg`) et `repo` :
`agent_backup_repo_reachable` (1 quand la dernière lecture a réussi, 0
sinon : dépôt inaccessible, mot de passe refusé, binaire manquant —
l'agent journalise la raison une fois, et à nouveau quand elle change),
`agent_backup_repo_last_snapshot_age_seconds` (âge du snapshot le plus
récent, absent tant que le dépôt est vide), et
`agent_backup_repo_last_snapshot_bytes` (octets que le snapshot restic le
plus récent a traités, restic 0.17 et plus récent). Avec `--latest 1`,
restic renvoie le snapshot le plus récent de chaque ensemble hôte et
chemin ; le plus récent de tous est gardé, réglez donc `host` sur un
dépôt partagé par plusieurs machines.

Règles intégrées : **Sauvegarde restic/Borg trop ancienne** (aucun
snapshot depuis plus de 48 heures) et **Dépôt restic/Borg inaccessible**
(depuis 30 minutes). La page de l'appareil montre un panneau Dépôts de
sauvegarde, une ligne par dépôt.

Pour vérifier une déclaration, lancez l'agent une fois au premier plan et
cherchez les échantillons `agent_backup_repo_*` :

```sh
sudo dumbmonit-agent --config /etc/dumbmonit/agent.yaml --dry-run
```

## Tunnels WireGuard {#wireguard-tunnels}

Sur une machine qui fait tourner WireGuard, l'agent lit chaque interface
et chaque pair avec `wg show all dump`, à chaque cycle. Rien à
configurer : sans la commande `wg` (wireguard-tools) ou sans interface,
aucune série n'est émise et aucun panneau montré. `wg` a besoin de root
(`CAP_NET_ADMIN`) ; le service installé l'a, un agent démarré à la main
sous votre propre compte journalise le refus une fois et ne rapporte
rien.

Le dump porte aussi la clé privée de l'interface et la clé préparée de
chaque pair : l'agent saute les deux champs sans jamais les copier.

Par pair, étiqueté `interface`, `peer` (sa clé publique) et
`allowed_ips` : `agent_wireguard_peer_handshake_age_seconds` (secondes
depuis le dernier handshake), `agent_wireguard_peer_has_handshake` (0
tant que le pair n'en a jamais terminé un), `agent_wireguard_peer_keepalive_seconds`
(son keepalive persistant, 0 quand désactivé), et les compteurs
`agent_wireguard_peer_rx_bytes` et `agent_wireguard_peer_tx_bytes` ; par
interface, `agent_wireguard_interface_peers`. Pour un pair qui n'a jamais
terminé de handshake, l'âge compte depuis le moment où l'agent l'a vu
pour la première fois : un tunnel qui n'est jamais monté est silencieux
aussi, et rapporté comme tel.

WireGuard n'a pas de noms de pair. Donnez les vôtres dans `agent.yaml`,
par clé publique, et ils deviennent le label `name` :

```yaml
wireguard: true                     # défaut ; false arrête même de regarder
wireguard_bin: wg                   # défaut : sur PATH
wireguard_peer_names:
  "zonmO992nrIL4FgJ3B/wccmu/SBkoX2kDhSYr8KpgRU=": office-gateway
  "UsKpqhrXNlYy/+rh2DLAOOEqesn8L8okEwNNorZ09lQ=": laptop
```

La règle intégrée **Pair WireGuard silencieux** se déclenche quand un
pair **avec un keepalive persistant** n'a pas terminé de handshake depuis
plus de 15 minutes. Tandis qu'un tunnel est monté, WireGuard renouvelle
son handshake toutes les deux minutes, et un keepalive le garde monté
même sans trafic ; un pair sans keepalive (un téléphone, un portable) peut
rester inactif des jours sans rien de cassé, il est donc laissé de côté.
Réglez `PersistentKeepalive = 25` sur les tunnels qui doivent rester
montés (liens site à site, passerelles distantes). N est le seuil de la
règle : changez-le dans **Alertes → Règles**, ou retirez la partie
`and on (…)` de sa requête pour surveiller chaque pair. La page de
l'appareil montre un panneau WireGuard avec chaque pair, le temps depuis
son dernier handshake, son trafic et son keepalive.

## Certificats TLS (Certbot / Let's Encrypt)

L'agent ne lit pas `/etc/letsencrypt` directement. Chaque certificat que
Certbot renouvelle est servi quelque part — un serveur web, un serveur
mail, un reverse proxy — et c'est là que DumbMonit le lit : ajoutez un
moniteur [Certificat TLS](services.md#tls) (ou un
[Site web ou API web](services.md#http), pour un site déjà surveillé
ainsi) pointé sur le port où le certificat est réellement présenté. Il
lit le certificat en direct, pas un fichier sur disque, il attrape donc
aussi un renouvellement qui a réussi dans `/etc/letsencrypt` mais n'a
jamais été pris en compte par le service (un reverse proxy qui a besoin
d'un reload pour voir le nouveau fichier est le cas habituel, et
exactement l'écart qui compte).

Lire `/etc/letsencrypt/live/*/cert.pem`, la configuration de
renouvellement et le journal de Certbot depuis l'agent a été envisagé et
écarté : ça dupliquerait la date d'expiration que le moniteur TLS
rapporte déjà, tout en ajoutant un second chemin, local à l'hôte, pour la
même information — une chose de plus à garder synchronisée après qu'un
certificat est déplacé ou un domaine renommé. Un hôte où Certbot tourne
mais ne sert rien que DumbMonit peut atteindre directement (une étape de
CA interne, un hook de validation DNS) est le seul cas que ça laisse non
couvert ; c'est aussi le cas où un moniteur TLS ne pourrait pas aider non
plus, puisque rien sur cet hôte ne parle TLS pour être sondé.

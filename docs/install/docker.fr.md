# Installation avec Docker {#install-with-docker}

DumbMonit tourne dans un seul conteneur : le serveur `dumbmonit` (collecte, API,
alertes, interface web) démarre son propre VictoriaMetrics pour le stockage des séries temporelles ; le
binaire est fourni dans l'image. La configuration et l'état vivent dans une base SQLite
embarquée. Un seul volume, `/data`, contient la base, le secret de l'instance et les
séries temporelles.

!!! note "Image alpha"
    `ghcr.io/laupernoe/dumbmonit:latest` est le dernier build alpha étiqueté (amd64 ;
    les images arm64 sont suspendues pour l'instant) ; `:edge` suit le dernier commit de `main`. Pour exécuter depuis les sources,
    `docker compose up -d --build` construit la même image en local (environ dix
    minutes la première fois ; seul Docker est nécessaire).

## Prérequis {#prerequisites}

- Docker Engine avec le plugin Compose (`docker compose version` fonctionne).
- Une machine qui peut joindre les appareils à surveiller. Elle n'a pas besoin d'être
  joignable depuis eux, sauf pour les [agents](../devices/agent.md), qui poussent
  leurs mesures vers le serveur en HTTP.
- Le port `8080` libre sur l'hôte, ou un autre port de votre choix (voir plus bas).

## Le fichier Compose {#the-compose-file}

Voici le `docker-compose.yml` du dépôt ; enregistrez-le dans un répertoire
dédié :

```yaml
# Un conteneur : le serveur DumbMonit exécute son propre VictoriaMetrics (embarqué dans
# l'image) et garde tout dans un seul volume. Pour utiliser un VictoriaMetrics
# externe, définissez DUMBMONIT_VM_URL et l'embarqué n'est pas démarré.
name: dumbmonit

services:
  dumbmonit:
    # `docker compose up -d` récupère l'image publiée (dernière version ; utilisez
    # `:edge` pour le dernier commit de main). Pour construire depuis ce dépôt,
    # lancez `docker compose up -d --build` : le résultat porte le même nom
    # et est utilisé ensuite.
    image: ghcr.io/laupernoe/dumbmonit:latest
    build: .
    ports:
      # Le port de l'hôte est configurable : 8080 est un port très sollicité sur une
      # machine de homelab. `DUMBMONIT_PORT=8099 docker compose up -d` le déplace.
      - "${DUMBMONIT_PORT:-8080}:8080"
    volumes:
      # Base SQLite, secret de l'instance et séries temporelles (/data/vm).
      #
      # Le serveur tourne sous l'utilisateur 65532 (pas root). Un volume nommé créé par
      # Docker hérite de ce propriétaire depuis l'image : rien à faire. Un bind mount
      # (`./data:/data`) ou un volume créé avant ce changement doit être
      # transféré une fois :
      #   docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
      # ou, pour garder les fichiers tels quels, exécutez le conteneur sous leur propriétaire
      # avec `user: "1000:1000"` (tout uid convient : l'image n'a pas de /etc/passwd).
      - dumbmonit-data:/data
    environment:
      # Décommentez pour définir vous-même le secret au lieu de laisser DumbMonit
      # le générer dans /data/secret.key. Il chiffre les identifiants des appareils : le
      # perdre oblige à les ressaisir tous.
      # DUMBMONIT_SECRET: replace-me-with-32-random-characters
      # Mot de passe perdu : `DUMBMONIT_RESET_PASSWORD=1 docker compose up -d` efface
      # le mot de passe au démarrage et l'interface en demande un nouveau ; relancez ensuite
      # sans la variable.
      DUMBMONIT_RESET_PASSWORD: ${DUMBMONIT_RESET_PASSWORD:-}
      # VictoriaMetrics embarqué : rétention (mois, ou p. ex. 30d / 2y) et budget
      # mémoire de ses caches. Un homelab de quelques dizaines d'appareils tient dans
      # 256 Mo ; augmentez-le pour des centaines d'hôtes.
      DUMBMONIT_VM_RETENTION: ${DUMBMONIT_VM_RETENTION:-12}
      DUMBMONIT_VM_MEMORY: ${DUMBMONIT_VM_MEMORY:-256MB}
      # VictoriaMetrics externe à la place de l'embarqué :
      # DUMBMONIT_VM_URL: http://victoriametrics:8428
    # Moindre privilège : aucune capability, pas d'élévation de privilèges, et le
    # système de fichiers de l'image en lecture seule — /data (volume) et /tmp (tmpfs) sont les
    # seuls emplacements inscriptibles. Les moniteurs « ping » ICMP n'ont besoin d'aucune capability non plus : le
    # sysctl ci-dessous permet au serveur (sans privilèges) d'ouvrir des sockets ICMP echo dans
    # l'espace de noms réseau du conteneur. Retirez-le si vous n'utilisez jamais le ping.
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
    read_only: true
    tmpfs:
      - /tmp
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
    # Le serveur arrête VictoriaMetrics après lui-même : laissez-lui le temps de le faire.
    stop_grace_period: 30s
    restart: unless-stopped

volumes:
  dumbmonit-data:
    # Nom fixe, indépendant du nom du projet compose : le volume est ce que
    # vous sauvegardez et ce qu'une mise à jour doit retrouver.
    name: dumbmonit-data
```

Puis démarrez-le :

```bash
docker compose up -d
```

!!! tip "Construire l'image vous-même"
    `docker compose up -d` récupère l'image publiée et ignore `build: .`.
    Depuis un clone du dépôt, `docker compose up -d --build` construit la
    même image en local à la place (environ dix minutes à froid ; aucune chaîne d'outils Rust ou Node
    n'est nécessaire sur l'hôte). Utilisez-le pour exécuter depuis les sources ou depuis une
    branche.

## VictoriaMetrics embarqué {#embedded-victoriametrics}

L'image contient le binaire VictoriaMetrics (`/victoria-metrics-prod`, issu de
`victoriametrics/victoria-metrics:v1.152.0`). Lorsque `DUMBMONIT_VM_URL` n'est pas
défini, le serveur le démarre comme processus enfant à l'écoute sur `127.0.0.1:8428`,
stocke ses séries sous `/data/vm`, redirige ses lignes de journal vers le sien,
le redémarre avec un délai croissant s'il meurt et l'arrête à l'extinction. Rien n'est
publié : le port reste à l'intérieur du conteneur.

Deux variables sont à connaître : `DUMBMONIT_VM_RETENTION` (`12` mois par
défaut ; `30d` ou `2y` fonctionnent aussi) et `DUMBMONIT_VM_MEMORY` (`256MB`, le
budget de ses caches ; augmentez-le pour des centaines d'hôtes). Le reste se trouve dans la
[référence de configuration](../reference/configuration.md#embedded-victoriametrics).

Pour utiliser un VictoriaMetrics que vous exploitez déjà, définissez `DUMBMONIT_VM_URL` à son
adresse (`http://host:8428`) : l'embarqué n'est alors pas démarré et
`/data/vm` reste vide.

## Premier démarrage {#first-start}

Ouvrez `http://<votre-hôte>:8080`. Une instance vierge affiche l'écran `/setup`,
où vous créez le premier compte administrateur (mot de passe d'au moins 12
caractères).

L'écran demande d'abord le **code d'installation**. Tant qu'aucun administrateur n'existe,
le serveur affiche un code à usage unique dans ses journaux au démarrage :

```sh
docker compose logs dumbmonit | grep -A1 "setup code"
```

```
  First-run setup code: K7XQ4-M9PRT
```

Il prouve que la personne qui crée le compte admin exploite le serveur, de sorte que quelqu'un
d'autre sur le réseau ne puisse pas s'emparer d'abord d'une instance vierge. Le code ne vit
qu'en mémoire, change à chaque redémarrage tant qu'un admin n'existe pas, et les
tentatives erronées sont limitées en débit. Pour un déploiement automatisé, définissez
`DUMBMONIT_SETUP_CODE` pour le choisir vous-même.

![L'écran de connexion](../assets/screenshots/login-light.png){ loading=lazy }

Ajoutez ensuite votre premier appareil : voir [Ajouter votre premier appareil](first-device.md).

## Variables d'environnement {#environment-variables}

Tout passe par des variables d'environnement ; aucune n'est obligatoire.

| Variable | Défaut | Rôle |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Adresse d'écoute à l'intérieur du conteneur. |
| `DUMBMONIT_DATA_DIR` | `/data` | Base SQLite (`dumbmonit.db`), secret de l'instance (`secret.key`) et données du VictoriaMetrics embarqué (`vm/`). |
| `DUMBMONIT_VM_URL` | *(non défini)* | URL d'un VictoriaMetrics externe. Lorsqu'elle est définie, l'embarqué n'est pas démarré. |
| `DUMBMONIT_VM_RETENTION` | `12` | Rétention du VictoriaMetrics embarqué : mois, ou `30d`, `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Budget mémoire des caches du VictoriaMetrics embarqué. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Adresse d'écoute du VictoriaMetrics embarqué, à l'intérieur du conteneur. |
| `DUMBMONIT_SECRET` | *(généré)* | Secret de l'instance qui chiffre les identifiants des appareils et les jetons. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Sondes simultanées, tous collecteurs confondus. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Durée maximale d'une sonde. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Période d'écriture vers VictoriaMetrics. |
| `DUMBMONIT_LOG` | `info` | Filtre de journalisation (syntaxe `tracing`, p. ex. `debug`, `dumbmonit=trace`). |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Binaires d'agent servis sous `/download/…`. |
| `DUMBMONIT_RESET_PASSWORD` | *(vide)* | Définir à `1` pour effacer le mot de passe et toutes les sessions au démarrage. |
| `DUMBMONIT_SETUP_CODE` | *(généré)* | Code d'installation demandé par `/setup` tant qu'aucun admin n'existe. Non défini, un code aléatoire est affiché dans les journaux à chaque démarrage. |
| `DUMBMONIT_COOKIE_SECURE` | *(désactivé)* | Définir à `1` derrière un reverse proxy TLS pour marquer le cookie de session `Secure`. |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Période d'évaluation des alertes (jamais en dessous de 10). |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | 90 jours | Rétention de l'historique des alertes. Voir la [référence de configuration](../reference/configuration.md) pour une réserve sur son unité. |
| `DUMBMONIT_BACKUP_ENABLED` | activé | Sauvegardes locales planifiées de la base dans `/data/backups/`. `DUMBMONIT_BACKUP_DIR`, `_INTERVAL_HOURS` (24) et `_KEEP` (7) les ajustent ; voir [Sauvegarde et restauration](backup.md). |

La liste complète, avec les détails, se trouve dans la [référence de configuration](../reference/configuration.md).
Les noms `EZYMONIT_*` d'avant le renommage sont toujours lus en repli ; voir
[Mise à jour](#upgrading).

## Sauvegarder la clé secrète {#back-up-the-secret-key}

Les communautés SNMP, mots de passe et jetons d'API sont chiffrés en AES-256-GCM avec une
clé dérivée du secret de l'instance. Au premier démarrage, DumbMonit génère ce
secret dans `/data/secret.key` (dans le volume `dumbmonit-data`).

!!! danger "Sauvegardez `secret.key` avec la base de données"
    Sans elle, les identifiants des appareils sont irrécupérables : une base restaurée
    seule donne une instance incapable de parler à quoi que ce soit. Le serveur détecte une clé
    manquante ou modifiée au démarrage et refuse de continuer avec un message explicite,
    plutôt que d'échouer silencieusement à chaque sonde. Les sauvegardes planifiées
    la copient pour vous — voir [Sauvegarde et restauration](backup.md).

Si vous préférez posséder le secret, définissez `DUMBMONIT_SECRET` dans le fichier Compose
à la place (32 caractères aléatoires ou plus). Gardez-le dans votre gestionnaire de mots de passe.

## Mise à jour {#upgrading}

```bash
docker compose pull
docker compose up -d
```

Les migrations de base de données s'exécutent au démarrage. Les données de VictoriaMetrics restent intactes.

!!! tip "Faites d'abord une sauvegarde"
    **Settings → Backup → Back up now** écrit en quelques secondes une copie cohérente de la base
    et de `secret.key` dans `/data/backups/`, et c'est de là que vous
    restaurez si la mise à jour tourne mal. Voir
    [Sauvegarde et restauration](backup.md).

### Depuis 0.1.0-alpha.1 {#from-010-alpha1}

Depuis 0.1.0-alpha.2, le conteneur tourne sous l'utilisateur 65532 au lieu de root. Un
volume créé par alpha.1 appartient toujours à root, et le serveur refuse de
démarrer (`/data is not writable by the server`). Transférez le volume une fois :

```bash
docker compose down
docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
docker compose up -d
```

Ou gardez les fichiers tels quels et exécutez le conteneur sous leur propriétaire, avec
`user: "0:0"` (ou l'uid d'un bind mount) sous le service dans
`docker-compose.yml`.

### Depuis EzyMonit, et depuis l'installation à deux conteneurs {#from-ezymonit-and-from-the-two-container-setup}

DumbMonit s'appelait EzyMonit jusqu'en septembre 2026, et tournait dans deux conteneurs,
le serveur et un service `victoriametrics` séparé. La mise à jour demande de déplacer une fois les données
de l'ancien projet dans le nouveau volume. L'ancien projet s'appelait
`ezymonit` (ses volumes `ezymonit_ezymonit-data` et `ezymonit_vm-data` ;
`docker volume ls` le confirme). Depuis l'ancien répertoire :

```bash
docker compose down
```

Puis, avec le nouveau `docker-compose.yml` :

```bash
docker volume create dumbmonit-data
docker run --rm -v ezymonit_ezymonit-data:/from -v dumbmonit-data:/to alpine cp -a /from/. /to/
docker run --rm -v ezymonit_vm-data:/from -v dumbmonit-data:/to alpine sh -c 'mkdir -p /to/vm && cp -a /from/. /to/vm/'
docker compose up -d
```

L'ancien répertoire de données de VictoriaMetrics a la structure qu'utilise l'embarqué :
rien à convertir. L'alternative est de garder le VictoriaMetrics externe et
de pointer `DUMBMONIT_VM_URL` vers lui.

Ce que le serveur gère seul au premier démarrage :

- Les variables `EZYMONIT_*` sont toujours lues en repli, avec un avertissement au démarrage
  par variable (`EZYMONIT_X is deprecated, use DUMBMONIT_X`).
- `/data/ezymonit.db` est renommé en `dumbmonit.db`.
- Les jetons d'agent `ezym_…` continuent de fonctionner ; les nouveaux sont `dmon_…`. Les agents continuent
  de pousser ; relancez la commande d'installation sur chaque machine quand cela vous arrange, elle
  [migre l'ancien service sur place](../devices/agent.md#upgrade).
- Le cookie de session a changé de nom : tout le monde se reconnecte une fois. La préférence de thème
  du navigateur est conservée.

Ce qu'il ne gère pas : le préfixe des métriques est passé de `ezymonit_` à
`dumbmonit_` sans compatibilité. Les anciennes séries restent dans VictoriaMetrics sous
l'ancien nom et disparaissent avec la rétention ; les graphiques et les règles intégrées repartent
à partir de la mise à jour. Les règles d'alerte personnalisées qui citent des métriques `ezymonit_…` doivent être
modifiées.

## Sauvegarde et restauration {#backup-and-restore}

Un seul volume nommé, `dumbmonit-data`, contient tout :

| Chemin | Contenu |
|---|---|
| `dumbmonit.db` | Appareils, règles, canaux, état des alertes, sessions. |
| `secret.key` | Le secret de l'instance. |
| `backups/` | Les sauvegardes locales planifiées : une copie de la base et du secret, quotidienne par défaut, sept conservées. |
| `vm/` | Séries temporelles de VictoriaMetrics (12 mois de rétention par défaut). |

DumbMonit sauvegarde lui-même sa base selon un calendrier, et exporte toute la
configuration — appareils, identifiants, règles, canaux — dans un seul fichier
chiffré avec une phrase secrète de votre choix, qui se restaure sur une instance vierge.
Les deux sont décrits dans **[Sauvegarde et restauration](backup.md)**, avec ce qu'il faut faire avant
une mise à jour et l'unique règle sur `secret.key` qui mérite d'être lue deux fois.

Pour archiver le volume lui-même, graphiques compris, arrêtez la pile et archivez-le avec tar :

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data -v "$PWD:/backup" alpine \
  tar czf /backup/dumbmonit-data.tgz -C /data .
docker compose start
```

Pour restaurer, créez le volume, extrayez-y l'archive de la même façon, puis
`docker compose up -d`.

Si la place compte, `--exclude=./vm` garde l'archive petite : la base et
le secret constituent la configuration, `vm/` ne contient que les graphiques.

## Changer le port {#changing-the-port}

Définissez `DUMBMONIT_PORT` au démarrage :

```bash
DUMBMONIT_PORT=8099 docker compose up -d
```

Ou placez `DUMBMONIT_PORT=8099` dans un fichier `.env` à côté du fichier Compose.

## Reverse proxy {#reverse-proxy}

L'interface et l'API sont servies sur un seul port en HTTP simple. Aucun WebSocket n'est utilisé :
l'interface interroge l'API périodiquement, donc n'importe quel reverse proxy fonctionne sans configuration
particulière.

=== "Caddy"

    ```
    monit.example.com {
        reverse_proxy 127.0.0.1:8080
    }
    ```

=== "nginx"

    ```nginx
    server {
        listen 443 ssl;
        server_name monit.example.com;
        # ssl_certificate / ssl_certificate_key …

        location / {
            proxy_pass http://127.0.0.1:8080;
            proxy_set_header Host $host;
            proxy_set_header X-Forwarded-Proto $scheme;
            # Les agents envoient des lots de rattrapage après une panne :
            client_max_body_size 16m;
        }
    }
    ```

Lorsque le proxy termine le TLS, définissez `DUMBMONIT_COOKIE_SECURE: "1"` pour que le cookie de session
ne soit envoyé que par HTTPS. Ne le définissez pas sur un déploiement en HTTP simple : le
navigateur ne renverrait jamais le cookie et la connexion serait impossible.

!!! note "Agents derrière un proxy"
    La commande d'installation affichée lorsque vous créez un jeton d'agent utilise l'URL que votre
    navigateur a employée pour joindre l'interface. Si c'est l'URL du proxy, les agents l'utiliseront
    aussi : assurez-vous que `/install.sh`, `/install.ps1`, `/download/…` et `/api/ingest`
    passent bien par le proxy.

## Exécution sans privilèges root {#runs-as-a-non-root-user}

Le conteneur exécute le serveur sous l'utilisateur `65532:65532` (numérique : l'image `scratch`
n'a pas de `/etc/passwd`), avec toutes les capabilities retirées, `no-new-privileges`
et un système de fichiers racine en lecture seule ; `/data` (le volume) et `/tmp` (un tmpfs)
sont les seuls chemins inscriptibles. Tout cela figure dans le fichier Compose ci-dessus.

Un volume nommé créé par Docker au premier démarrage hérite du propriétaire de `/data`
depuis l'image : rien à faire. Deux cas demandent une commande :

- **Un volume créé par une version antérieure** (le serveur tournait auparavant sous root,
  donc les fichiers appartiennent à root). Transférez-les une fois, pile arrêtée :

  ```bash
  docker compose stop
  docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
  docker compose up -d
  ```

  Sinon, le symptôme est un conteneur qui s'arrête aussitôt avec
  `creating directory /data … Permission denied` (ou `unable to open database
  file`).

- **Un bind mount** (`./data:/data`) conserve le propriétaire du répertoire de l'hôte.
  Soit `chown -R 65532:65532 ./data`, soit exécutez le conteneur sous le propriétaire du
  répertoire : `user: "1000:1000"` sous le service, tout uid convient.

## Ping ICMP sans capability {#icmp-ping-without-a-capability}

Le [moniteur ping](../devices/services.md#ping) envoie des ICMP echo. Plutôt qu'un
socket brut — qui demande `NET_RAW`, et une capability accordée à un utilisateur non root
du conteneur n'est de toute façon pas utilisable — il emploie un socket ICMP echo, que Linux
autorise pour les groupes listés dans `net.ipv4.ping_group_range`. Le fichier Compose
définit ce sysctl dans l'espace de noms réseau propre au conteneur :

```yaml
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Rien ne change sur l'hôte. Docker 20.10 et les versions ultérieures définissent d'eux-mêmes cette plage dans chaque
conteneur ; les lignes explicites la garantissent sur les moteurs plus anciens et
sur Podman. Sans elle, la vérification signale une erreur de configuration (affichée sur
l'appareil, sans notification), jamais un faux « hôte hors service ». Avec `docker run`, passez
`--sysctl net.ipv4.ping_group_range="0 2147483647"`.

## Tester sans matériel {#testing-without-hardware}

Ajoutez un [appareil de démonstration](../devices/demo.md) : il produit de fausses mesures sans
rien à préparer, de sorte que les graphiques, règles et notifications peuvent être essayés
immédiatement. Pour de vraies données SNMP, la machine qui fait tourner Docker suffit souvent — installez
`snmpd` dessus (`apt install snmpd`, autorisez la communauté `public` sur le pont Docker
dans `/etc/snmp/snmpd.conf`) et ajoutez un appareil SNMP avec l'adresse de l'hôte
sur ce pont (`172.17.0.1` par défaut) : le profil est détecté
automatiquement et les interfaces, la mémoire et les processus apparaissent.

Les développeurs peuvent ajouter la surcouche `docker-compose.dev.yml` pour publier le VictoriaMetrics
embarqué sur le `:8428` de l'hôte pour des requêtes MetricsQL directes :

```bash
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d
```

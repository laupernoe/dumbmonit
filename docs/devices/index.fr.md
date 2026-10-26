# Types de surveillance

DumbMonit appelle tout ce qu'il surveille une *surveillance* (l'API dit *target*),
même quand c'est une page web ou un nom DNS. Chaque type est un collecteur
sur le serveur ; le serveur décrit chaque type à l'interface
(`GET /api/collectors`), si bien que le formulaire correspond toujours à ce
que cette version peut surveiller.

| Type | `kind` | Identifiant | Usage typique |
|---|---|---|---|
| [Appareil SNMP](snmp.md) | `snmp` | communauté ou SNMP v3 | Commutateur, routeur, NAS, onduleur, imprimante |
| [Proxmox VE](proxmox.md) | `proxmox` | jeton d'API ou nom d'utilisateur/mot de passe | Hyperviseur, cluster |
| [Proxmox Backup Server](pbs.md) | `pbs` | jeton d'API ou nom d'utilisateur/mot de passe | Serveur de sauvegarde |
| [Proxmox Datacenter Manager](pdm.md) | `pdm` | jeton d'API | Console fédérant plusieurs clusters PVE et instances PBS |
| [Proxmox Mail Gateway](pmg.md) | `pmg` | nom d'utilisateur/mot de passe | Passerelle mail filtrant spam et virus |
| [VMware vSphere](vsphere.md) | `vsphere` | nom d'utilisateur/mot de passe | vCenter ou ESXi autonome : hôtes, VM, datastores, alarmes |
| [XCP-ng / XenServer](xcpng.md) | `xcpng` | nom d'utilisateur/mot de passe (rôle XAPI en lecture seule) | Pool via XAPI : hôtes, VM, référentiels de stockage, haute disponibilité |
| [Nutanix AHV](nutanix.md) | `nutanix` | nom d'utilisateur/mot de passe (Prism Viewer) | API v4 de Prism Central : clusters, hôtes, VM AHV, conteneurs de stockage, alertes |
| [Synology DSM](synology.md) | `synology` | nom d'utilisateur/mot de passe | DiskStation, RackStation |
| [OPNsense](opnsense.md) | `opnsense` | clé et secret d'API | Pare-feu, routeur, bordure multi-WAN |
| [pfSense](pfsense.md) | `pfsense` | clé d'API REST (utilisateur en lecture seule) | Pare-feu : passerelles en panne ou avec pertes, interfaces sans lien, services arrêtés |
| [FortiGate](fortigate.md) | `fortigate` | jeton d'API REST (profil en lecture seule, hôte de confiance) | Pare-feu : tunnels IPsec, liens, synchro HA, mode conserve, licences FortiGuard |
| [Sophos Firewall](sophos.md) | `sophos` | nom d'utilisateur/mot de passe (profil en lecture seule) | Pare-feu : liens d'interface seulement (l'API XML n'expose ni tunnel, ni HA, ni licence) |
| [Cisco Meraki](meraki.md) | `meraki` | clé d'API (administrateur en lecture seule) | Bêta. Cloud Meraki : appareils hors ligne, liaisons Internet en échec, état et expiration de la licence, clients par réseau |
| [TP-Link Omada](omada.md) | `omada` | identifiant et secret de client OpenAPI (rôle Viewer) | Bêta. Contrôleur Omada : appareils déconnectés ou en attente, CPU et mémoire, mises à jour du firmware, clients par site |
| [Aruba Central](arubacentral.md) | `arubacentral` | client d'API (ID et secret) ou jeton d'accès | Bêta. Points d'accès, commutateurs et passerelles Aruba par l'API cloud : appareils hors ligne, alertes critiques, santé des appareils, clients |
| [Palo Alto PAN-OS](paloalto.md) | `paloalto` | nom d'utilisateur/mot de passe ou clé d'API (rôle limité aux requêtes opérationnelles de l'API XML) | Bêta. Pare-feu : liens d'interface, pair et synchro HA, table des sessions, licences, validations en échec |
| [SonicWall SonicOS](sonicwall.md) | `sonicwall` | nom d'utilisateur/mot de passe (utilisateur en lecture seule) | Bêta. Pare-feu : liens d'interface seulement |
| [Serveur OpenVPN](openvpn.md) | `openvpn` | aucun, ou mot de passe de gestion | Serveur OpenVPN via son interface de gestion : version, état, clients connectés |
| [UniFi Network](unifi.md) | `unifi` | clé d'API ou nom d'utilisateur/mot de passe | Console UniFi ou serveur auto-hébergé : appareils, WAN, clients |
| [Home Assistant](homeassistant.md) | `homeassistant` | jeton d'accès longue durée | Maison connectée : entités indisponibles, batteries faibles, mises à jour, réparations |
| [TrueNAS](truenas.md) | `truenas` | clé d'API | Serveur de stockage ZFS (SCALE, Community Edition) |
| [Unraid](unraid.md) | `unraid` | clé d'API (rôle Viewer) | NAS : array, disques, vérifications de parité, pools de cache, conteneurs, VM |
| [QNAP QTS / QuTS hero](qnap.md) | `qnap` | nom d'utilisateur/mot de passe (compte limité) | NAS (bêta) : volumes qui se remplissent, état SMART et température des disques, mises à jour |
| [Ceph](ceph.md) | `ceph` | utilisateur et mot de passe du tableau de bord (rôle read-only) | Grappe de stockage (bêta) : contrôles de santé, OSD, groupes de placement, capacité, pools, moniteurs |
| [MinIO](minio.md) | `minio` | jeton Prometheus (ou aucun) | Stockage objet (bêta) : nœuds et disques hors ligne, capacité, échecs de réparation et de réplication, erreurs S3 |
| [MikroTik RouterOS](mikrotik.md) | `mikrotik` | nom d'utilisateur/mot de passe (groupe en lecture seule) | Routeur ou commutateur : versions et firmware en retard, CPU, mémoire, capteurs, erreurs d'interface |
| [Onduleur avec NUT](nut.md) | `nut` | aucun, ou nom d'utilisateur/mot de passe | Onduleur sur un NAS, un Raspberry Pi ou un serveur faisant tourner Network UPS Tools |
| [Matériel serveur (Redfish)](redfish.md) | `redfish` | nom d'utilisateur/mot de passe | Ventilateurs, températures, alimentations et disques du serveur, lus depuis son BMC |
| [VictoriaMetrics](victoriametrics.md) | `victoriametrics` | aucun, nom d'utilisateur/mot de passe ou jeton | Base de séries temporelles : ingestion, échantillons refusés, marge de disque |
| [VictoriaLogs](victoriametrics.md#let-dumbmonit-read-victorialogs-own-health) | `victorialogs` | aucun, nom d'utilisateur/mot de passe ou jeton | Base de logs : ingestion, lignes refusées, marge de disque |
| [Grafana Loki](loki.md) | `loki` | aucun, nom d'utilisateur/mot de passe ou jeton | Serveur de logs : état de préparation, lignes refusées, échecs de flush |
| [Graylog](graylog.md) | `graylog` | jeton d'accès ou nom d'utilisateur/mot de passe | Serveur de logs et son cluster OpenSearch : journal, buffers, débit, inputs |
| [Pi-hole](pihole.md) | `pihole` | mot de passe d'application, ou aucun | Bloqueur de publicité DNS : état de blocage, part bloquée, âge des listes, mises à jour |
| [AdGuard Home](adguard.md) | `adguard` | nom d'utilisateur/mot de passe | Filtre DNS : protection, blocages, listes de filtres, serveurs amont, mises à jour |
| [MDaemon Email Server](mdaemon.md) | `mdaemon` | aucun (agent requis sur le serveur), ou adresse email/mot de passe | Serveur mail : files, sessions et services Windows lus par l’agent, plus SMTP, IMAP, POP3, webmail, version |
| [SecurityGateway for Email Servers](securitygateway.md) | `securitygateway` | aucun (agent requis sur le serveur), ou clé d’API | Passerelle mail : services Windows et files lus par l’agent, file de remise et autres compteurs |
| [Redis / Valkey](redis.md) | `redis` | utilisateur ACL, mot de passe seul, ou aucun | Stockage en mémoire : mémoire par rapport à maxmemory, réplication, persistance, clés |
| [MongoDB](mongodb.md) | `mongodb` | nom d'utilisateur/mot de passe (clusterMonitor) ou aucun | Base de données documents : replica set, retard, connexions, cache WiredTiger |
| [RabbitMQ](rabbitmq.md) | `rabbitmq` | nom d'utilisateur/mot de passe (étiquette monitoring) | Courtier de messages : alarmes de ressource, nœuds, partitions, files sans consommateur |
| [CrowdSec](crowdsec.md) | `crowdsec` | aucun, ou clé d'API bouncer | Moteur de sécurité : décisions, alertes, bouncers qui ont arrêté de récupérer, lecture de logs |
| [Traefik](traefik.md) | `traefik` | aucun, ou utilisateur basicAuth | Reverse proxy : routeurs désactivés, serveurs backend en panne, certificats non obtenus ou non renouvelés, part de 5xx |
| [Caddy](caddy.md) | `caddy` | utilisateur de vue en lecture seule, ou aucun | Serveur web et reverse proxy : santé des upstreams, rechargements échoués, erreurs de handler, part de 5xx |
| [Exporter Prometheus](prometheus.md) | `prometheus` | aucun, authentification basique ou jeton bearer | N'importe quel exporter (node_exporter, windows_exporter, cAdvisor, blackbox…) : toutes les séries enregistrées, tracées et utilisables dans des règles |
| [Source OpenTelemetry](otlp.md) | `otlp` | jeton d'API (portée write) | Récepteur OTLP/HTTP : métriques d'un OpenTelemetry Collector ou d'une application, journaux transmis à VictoriaLogs |
| [Nginx](nginx.md) | `nginx` | aucun, ou authentification basique | Serveur web : connexions actives/en attente, débits accept/handled/request, santé des upstreams NGINX Plus |
| [Apache httpd](apache.md) | `apache` | aucun, ou authentification basique | Serveur web : workers occupés/au repos, scoreboard par état, débits de requêtes et d'octets |
| [Nginx Proxy Manager](npm.md) | `npm` | email et mot de passe utilisateur (vue seule) | Gestionnaire de proxy : hôtes refusés par nginx, hôtes désactivés, certificats servis |
| [Expiration de domaine](domain.md) | `domain` | aucun | Domaine enregistré via RDAP : jours avant expiration, suspension, période de rachat, registrar |
| [État d’un fournisseur tiers](provider.md) | `provider` | aucun | Page de statut publique d’un fournisseur (Cloudflare, GitHub, Atlassian, AWS…) : impact, incidents ouverts, filtre par composant ; affiché sur les pages de statut comme composant externe |
| [Réputation du serveur mail](mail_blacklist.md) | `mail_blacklist` | aucun, ou clé DQS de Spamhaus | Domaine de messagerie ou adresse d’un serveur face aux listes noires DNS (Spamhaus, Barracuda, SpamCop…), DNS inverse, SPF, DMARC, liens de retrait |
| [Liste noire d’une plage d’IP](ip_blacklist.md) | `ip_blacklist` | aucun, ou clé DQS de Spamhaus | Chaque adresse d’une plage IPv4 publique jusqu’à /24 face aux listes noires DNS, avec liens de retrait |
| [Kubernetes / k3s](kubernetes.md) | `kubernetes` | jeton de compte de service (ClusterRole en lecture seule) | Cluster : nœuds non prêts, pods en boucle de crash ou en attente, workloads avec répliques manquantes, demandes de volume bloquées |
| [Active Directory](activedirectory.md) | `activedirectory` | nom d'utilisateur/mot de passe (utilisateur de domaine ordinaire) | Domaine Windows via LDAPS : contrôleurs de domaine et rôles FSMO, membres de groupes privilégiés, politique de mot de passe, constats de sécurité |
| [Veeam Backup & Replication](veeam.md) | `veeam` | nom d'utilisateur/mot de passe (Veeam Backup Viewer) | Serveur de sauvegarde : jobs en échec, sessions en échec, dépôts qui se remplissent, licence |
| [Kopia](kopia.md) (bêta) | `kopia` | nom d'utilisateur et mot de passe du serveur | Serveur de sauvegarde : dernier instantané de chaque source (âge, erreurs, taille), sources en retard sur leur planning, connexion au dépôt |
| [Duplicati](duplicati.md) (bêta) | `duplicati` | mot de passe de l'interface web | Serveur de sauvegarde (2.1+) : dernière sauvegarde réussie de chaque tâche, dernier résultat, avertissements et erreurs, tâche en cours |
| [Tailscale](tailscale.md) | `tailscale` | client OAuth (devices:core:read) ou jeton d'accès d'API | Tailnet : serveurs hors ligne, clés de nœud qui expirent, appareils en attente d'approbation |
| [Microsoft 365](m365.md) | `m365` | Application Entra (ID client et secret) | Santé des services, incidents, licences, utilisateurs à risque |
| [Microsoft Entra ID](entra.md) | `entra` | Application Entra (ID client et secret) | Expiration des secrets, synchronisation d'annuaire, MFA des administrateurs |
| [Microsoft Azure](azure.md) | `azure` | Application Entra (ID client et secret) avec le rôle Reader sur l'abonnement | Resource Health de chaque ressource et alertes Azure Monitor actives par sévérité |
| [AWS (CloudWatch)](aws.md) | `aws` | Identifiant de clé d'accès et secret (utilisateur IAM en lecture seule, rôle facultatif) | Alarmes CloudWatch en ALARM, contrôles d'état et CPU EC2, CPU et stockage libre RDS |
| [Cloudflare](cloudflare.md) | `cloudflare` | jeton d'API (lecture seule) | Zones, tunnels, certificats, erreurs 5xx, pools de répartition de charge |
| [Hetzner Cloud](hetzner.md) | `hetzner` | jeton d'API (lecture seule) | Serveurs, processeur, volumes, sauvegardes |
| [OVHcloud](ovh.md) | `ovh` | Jeton d'API OVHcloud (clé d'application, secret d'application et clé de consommateur) avec GET /vps* et GET /dedicated/server* | État et tâches en cours des VPS et des serveurs dédiés |
| [Infomaniak](infomaniak.md) | `infomaniak` | Jeton d'API Infomaniak avec la portée Products | Dates d'expiration, blocages et maintenance des domaines, hébergements et autres produits |
| [Nextcloud](nextcloud.md) | `nextcloud` | jeton de surveillance (NC-Token) | Synchronisation de fichiers : mode maintenance, mise à jour en attente, mises à jour, utilisateurs actifs, OPcache |
| [Immich](immich.md) | `immich` | clé d'API | Bibliothèque de photos : files de tâches, mises à jour, disque, nombre de photos |
| [Paperless-ngx](paperless.md) | `paperless` | jeton d'API ou nom d'utilisateur/mot de passe | Documents : Redis, Celery, index, imports en échec, mises à jour |
| [Jellyfin](jellyfin.md) | `jellyfin` | clé d'API | Serveur multimédia : tâches planifiées en échec, extensions, flux et transcodages |
| [Plex Media Server](plex.md) | `plex` | X-Plex-Token, ou aucun depuis un réseau autorisé | Serveur multimédia : flux, transcodages, bibliothèques, mises à jour |
| [GitLab (self-managed)](gitlab.md) | `gitlab` | jeton d'accès personnel | Forge : retard Sidekiq, runners CI, migrations en attente, réglages inscription/2FA, licence |
| [Forgejo / Gitea](forgejo.md) | `forgejo` | jeton d'accès | Forge : contrôle de santé, tâches planifiées, runners Actions, compteurs de dépôts/comptes |
| [Sonarr (bêta)](sonarr.md) | `sonarr` | clé d'API | Séries : santé, clients et indexeurs en échec, erreurs de file, disque, mises à jour |
| [Radarr (bêta)](radarr.md) | `radarr` | clé d'API | Films : santé, clients et indexeurs en échec, erreurs de file, disque, mises à jour |
| [Lidarr (bêta)](lidarr.md) | `lidarr` | clé d'API | Musique : santé, clients et indexeurs en échec, erreurs de file, disque, mises à jour |
| [Prowlarr (bêta)](prowlarr.md) | `prowlarr` | clé d'API | Indexeurs : santé, indexeurs désactivés après des échecs, disque, mises à jour |
| [qBittorrent (bêta)](qbittorrent.md) | `qbittorrent` | nom d'utilisateur et mot de passe, ou aucun | Client torrent : torrents en erreur, connectivité perdue, vitesses, espace libre |
| [UniFi Protect (bêta)](unifiprotect.md) | `unifiprotect` | nom d'utilisateur et mot de passe locaux | Caméras : hors ligne ou sans enregistrement, disques et stockage du NVR |
| [Frigate NVR (bêta)](frigate.md) | `frigate` | aucun ou jeton porteur | NVR : caméras sans images, détecteurs bloqués, débits, stockage des enregistrements |
| [Keycloak (bêta)](keycloak.md) | `keycloak` | aucun, ou identifiant et secret de client | Identité : disponibilité et base de données, realms, sessions, utilisateurs |
| [authentik (bêta)](authentik.md) | `authentik` | jeton d'API | Identité : disponibilité, workers, tâches en échec, avant-postes, mises à jour |
| [Vaultwarden (bêta)](vaultwarden.md) | `vaultwarden` | aucun, ou jeton d'administration | Coffre de mots de passe : disponibilité, version, inscriptions ouvertes, comptes |
| [mailcow (bêta)](mailcow.md) | `mailcow` | clé d'API en lecture seule | Serveur de messagerie : conteneurs arrêtés, file, volume des courriels, quotas |
| [Serveur avec agent](agent.md) | `agent` | aucun (jeton d'enrôlement) | Linux, Windows, Raspberry Pi, [hôte Hyper-V](agent.md#hyper-v-hosts) |
| [Site web ou API web](services.md#http) | `http` | aucun, nom d'utilisateur/mot de passe ou jeton | Page de santé, API REST |
| [Port réseau](services.md#tcp) | `tcp` | aucun | SSH, SMB, base de données |
| [Nom de domaine](services.md#dns) | `dns` | aucun | Votre domaine, un nom interne |
| [Hôte accessible](services.md#ping) | `ping` | aucun | Passerelle, point d'accès, imprimante |
| [Certificat TLS](services.md#tls) | `tls` | aucun | IMAPS, LDAPS, reverse proxy |
| [Serveur de temps NTP](services.md#ntp) | `ntp` | aucun | Routeur, NAS, serveur de temps interne, serveur de pool public |
| [Valeur SNMP](services.md#snmp-oid) | `snmp_oid` | communauté SNMP ou utilisateur v3 | Un objet SNMP : sonde, état d'onduleur, chaîne de version, avec bornes ou texte attendu |
| [Serveur RADIUS](services.md#radius) | `radius` | secret partagé et compte de test | FreeRADIUS, Windows NPS : une vraie authentification |
| [Service gRPC](services.md#grpc) | `grpc` | aucun | Microservice, etcd : le contrôle de santé gRPC standard |
| [Battement de cœur](push.md) | `push` | aucun (URL secrète) | Tâche cron, script de sauvegarde, automatisation qui doit appeler |
| [Changements de site web](webchange.md) | `webchange` | aucun | Une page ou tout un site : changements de texte, pages ajoutées et supprimées, avant/après avec captures d'écran |
| [Surveillance de démonstration](demo.md) | `dummy` | aucun | Explorer l'interface sans matériel |

## Comment une surveillance est lue

Le planificateur fait tourner chaque surveillance activée sur son propre
intervalle (60 s par défaut, 10 s au minimum) via le collecteur de son type,
avec au plus `DUMBMONIT_MAX_CONCURRENT_PROBES` sondes en vol et un délai
maximal de `DUMBMONIT_PROBE_TIMEOUT_SECS` par sonde. Chaque sonde réussie
écrit ses échantillons plus `dumbmonit_up = 1` ; une sonde en échec n'écrit
rien, et c'est le silence de la série que la règle « Surveillance inaccessible »
détecte.

Les échecs sont classés. Une surveillance qui ne répond pas est *inaccessible* et
déclenche une alerte. Une mauvaise communauté, un certificat refusé ou une
fonctionnalité manquante est une *erreur de configuration* : elle est
affichée sur la page de la surveillance, en rouge, avec la raison, et n'est pas
notifiée. La distinction compte : une surveillance mal configurée ne devrait
réveiller personne la nuit.

## Options

Certains types lisent des options depuis les étiquettes de la surveillance : le
port de l'API, s'il faut accepter un certificat autosigné, quels nœuds
surveiller. Le formulaire montre un champ typé par option sous
**Plus d'options**, avec son défaut et son texte d'aide. Les pages par type
les listent mot pour mot. Les options sont copiées sur chaque série comme
labels `tag_<key>`, ne mettez donc jamais un secret dans l'une d'elles :
utilisez le champ identifiant, qui est chiffré.

## Surveillances parentes

N'importe quelle surveillance peut déclarer une autre comme sa parente. Quand le
parent est inaccessible, les alertes de ses descendants sont marquées
*Supprimée par le parent* plutôt qu'envoyées : un commutateur qui tombe
produit une notification, pas trente. Sur la page Surveillances, les enfants
s'empilent sous leur parent et s'atténuent quand il est inaccessible.

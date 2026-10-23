# CVE et failles

**Sécurité → CVE et failles** liste les failles connues (CVE) des logiciels
installés sur vos équipements, et dit pour chacune si l'équipement est déjà
**corrigé**, encore **vulnérable**, ou **à vérifier** à la main. La page d'un
équipement montre la même chose pour lui seul.

!!! warning "Une invitation à regarder, pas un scanner de vulnérabilités"
    DumbMonit compare les **versions** qu'il relève déjà avec des avis
    publics. Il ne sonde pas les équipements, ne voit pas la configuration (un
    module vulnérable mais désactivé apparaît quand même) et ne connaît que
    les logiciels listés ci-dessous. Quand il ne peut pas trancher, il dit
    **à vérifier**, jamais **vulnérable**.

## L'activer

La recherche est **coupée par défaut** : tant qu'elle l'est, rien ne sort de
votre instance. Un administrateur l'active depuis la page elle-même. Une fois
active, l'inventaire logiciel (noms et versions des produits et des paquets)
est envoyé aux sources publiques ci-dessous ; aucun nom d'hôte, aucune adresse,
aucun identifiant n'est jamais transmis.

Facultatif : une **clé d'API NVD** (gratuite,
[à demander ici](https://nvd.nist.gov/developers/request-an-api-key)) fait
passer la limite de la NVD de 5 à 50 requêtes par 30 secondes : la première
recherche et les notes CVSS arrivent bien plus vite. Elle est gardée chiffrée
et n'est plus jamais affichée.

## Ce qui est inventorié

Rien n'est redemandé aux équipements : l'inventaire vient de ce que les
collecteurs relèvent déjà.

| Source | Logiciels |
| --- | --- |
| Proxmox VE | Version de Proxmox VE de chaque nœud, et les paquets listés par `apt/versions` (QEMU, LXC, Corosync, Ceph, swtpm, noVNC, et les paquets Debian de la liste) |
| Proxmox Backup Server, OPNsense, Unraid, VMware ESXi / vCenter, FortiGate (FortiOS), MikroTik (RouterOS), UniFi Network, Pi-hole, Nginx Proxy Manager, nginx, Apache, Traefik, MongoDB, Redis / Valkey, RabbitMQ, AdGuard Home, OpenVPN, Home Assistant, Kubernetes, MDaemon, SecurityGateway, Synology DSM, Veeam B&R, Immich, Forgejo, Jellyfin, Graylog | La version du produit remontée par son collecteur |
| Agent Linux | Tous les paquets installés, une fois par jour (`dpkg-query`, `rpm -qa` ou `apk list -I`) |
| Agent Windows | Édition, version et build de Windows (`10.0.22631.4317`), et programmes installés (Firefox, Chrome, Edge, 7-Zip, Notepad++, VLC, PuTTY, WinSCP, WinRAR, KeePass, Wireshark, FileZilla, TeamViewer, AnyDesk, OpenVPN, MDaemon…), une fois par jour |

TrueNAS, Proxmox Mail Gateway, Proxmox Datacenter Manager, XCP-ng, CrowdSec et
VictoriaMetrics sont inventoriés, mais aucune source ne les couvre encore. Le
panneau **Ce qui est couvert** liste chaque composant avec sa source.

L'inventaire quotidien de l'agent se coupe avec `inventory: false` dans
`agent.yaml` (ou `DUMBMONIT_AGENT_INVENTORY=false`).

## Sources

| Source | Sert à |
| --- | --- |
| [NVD](https://nvd.nist.gov/) (API 2.0) | Les produits, par CPE et plages de versions ; la note CVSS de chaque CVE |
| [Debian Security Tracker](https://security-tracker.debian.org/tracker/) | Les paquets Debian (et Raspberry Pi OS), et les paquets Debian des nœuds Proxmox : la version qui corrige chaque CVE **dans votre version de Debian**, rétroportages compris |
| [OSV.dev](https://osv.dev/) | Les paquets Ubuntu, Alpine, AlmaLinux et Rocky Linux |
| [CISA KEV](https://www.cisa.gov/known-exploited-vulnerabilities-catalog) | Marque les CVE **activement exploitées** |

Les sources sont relues **une fois par jour** (ou avec **Rafraîchir**), en
arrière-plan, jamais pendant le cycle de collecte. Tout est gardé en base : hors
ligne, le dernier état connu reste affiché et la source montre son erreur. La
NVD est lue produit par produit, puis seulement pour ce qui a changé
(`lastModStartDate`) ; le tracker Debian (environ 14 Mo compressé) est écrit sur
disque et lu en flux, seuls vos paquets sont gardés. Les requêtes sortantes
suivent les variables d'environnement `HTTPS_PROXY` / `HTTP_PROXY` /
`NO_PROXY` du conteneur.

This product uses the NVD API but is not endorsed or certified by the NVD.

## Verdicts

| Verdict | Sens |
| --- | --- |
| **Vulnérable** | La version installée est sous la version qui corrige, dans la plage touchée, ou la distribution la dit ouverte (pas encore de correctif) |
| **À vérifier** | Les données ne permettent pas de trancher : paquet reconstruit par l'éditeur (Proxmox `-pve`, révision de distribution) qui a pu recevoir le correctif, avis sans bornes de version, niveau de mise à jour d'ESXi, ou version illisible |
| **Corrigé** | La version installée contient déjà le correctif |

Les versions se comparent comme dans leur écosystème : ordre de `dpkg` pour les
paquets Debian (`1:9.2p1-2+deb12u3`), `époque:version-révision` pour les RPM,
ordre par segments pour les produits et les builds Windows.

Pour rester lisible, la liste garde tout ce qui est **exploité** ou **a un
correctif** ; une CVE sans correctif, ou à vérifier, d'un paquet de
distribution n'apparaît qu'à partir d'une note CVSS de 7 (et jamais quand
Debian la classe mineure) ; les CVE **corrigées** ne sont montrées que sur la
dernière année. Les notes CVSS des CVE de distribution arrivent de la NVD
progressivement, quelques centaines par heure sans clé.

## Alertes

Deux règles livrées, réglables comme les autres :

- **Exploited vulnerability not patched** (critique) : un composant est
  vulnérable à une CVE du catalogue KEV de la CISA.
- **Critical vulnerability not patched** (avertissement) : un composant est
  vulnérable à une CVE de note CVSS 9 ou plus.

Le reste est seulement affiché ; **à vérifier** n'alerte jamais. Les séries
sont `dumbmonit_cve_vulnerable{target,cve,component}` (valeur : note CVSS, 0 si
inconnue) et `dumbmonit_cve_exploited_vulnerable{target,cve,component}`.

## Ignorer (accepter le risque)

Un administrateur peut **ignorer** une CVE sur un équipement, avec une note
facultative (module inutilisé, correctif appliqué autrement…). Elle n'est
alors plus comptée et n'alerte plus ; elle reste listée, estompée, avec
l'auteur et la date. **Rétablir** annule.

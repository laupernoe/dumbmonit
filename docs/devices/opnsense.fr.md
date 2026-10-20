# OPNsense

Pare-feu et routeur open source : l'état de chaque passerelle avec son
temps d'aller-retour et sa perte de paquets, la table d'états pf, les
compteurs et adresses d'interface, les tunnels VPN, les baux DHCP, le
résolveur, les services, CARP et les mises à jour en attente.

!!! warning "Testé contre une instance de test seulement"

    Cette intégration a été construite et vérifiée contre OPNsense 26.1,
    l'image officielle démarrée dans une machine virtuelle pour
    l'occasion, avec la clé d'API restreinte décrite ci-dessous — pas
    contre un pare-feu qui route le trafic d'un foyer depuis des années.
    Chaque appel est une lecture et chaque forme vient de ce vrai
    OPNsense, mais ses passerelles n'avaient pas d'adresse de
    surveillance, il ne faisait pas partie d'une paire CARP et ne faisait
    tourner aucun tunnel OpenVPN ou IPsec : ces parties suivent le code
    source d'OPNsense plutôt qu'une réponse observée. Dites-nous ce qui
    casse.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | API (HTTPS, par défaut) |
| 80 | TCP | Seulement avec l'option de protocole HTTP |

## La page de la surveillance

Au-delà des graphiques génériques, une surveillance OPNsense montre trois
panneaux, lus depuis ce que la sonde a stocké — ouvrir la page n'interroge
jamais le pare-feu lui-même :

* **WAN et passerelles** — l'adresse publique actuelle de chaque interface
  qui porte une passerelle, puis chaque passerelle avec son état en un
  mot, son délai d'aller-retour et sa perte de paquets. Les passerelles en
  panne viennent d'abord. C'est le panneau pour lequel l'intégration
  existe : sur un pare-feu multi-WAN, le lien de secours prend le relais
  silencieusement et rien d'autre sur le réseau ne change.
* **Trafic et table d'états** — combien de connexions pf suit par rapport
  à sa limite configurée, puis chaque interface avec son état de lien, ses
  adresses, ses compteurs d'octets et de paquets, et ses erreurs et pertes
  quand il y en a. Puis les baux DHCP, comptés par serveur.
* **Santé du pare-feu** — les services arrêtés d'abord, puis les tunnels
  VPN avec le nombre de pairs connectés, puis CARP quand le pare-feu fait
  partie d'une paire, puis le firmware (version en cours, ce que le miroir
  propose, paquets en attente, et si un redémarrage est en attente), puis
  le résolveur, puis la machine : uptime, charge, mémoire, swap, buffers
  réseau, partitions et capteurs de température.

Compteurs seulement, jamais de trafic. DumbMonit ne lit aucune règle de
pare-feu, aucun état de connexion, aucune adresse de client et aucun nom
d'hôte. Les baux DHCP sont comptés, jamais listés ; les sessions VPN sont
comptées, jamais nommées.

## « none » signifie « en ligne »

`GET /api/routes/gateway/status` écrit `"none"` dans le champ `status`
d'une passerelle quand **tout va bien** — un mot qui seul se lit comme
« pas d'information ». DumbMonit le traduit en `online`, pour que la page,
les séries et les règles disent la même chose que la propre interface
d'OPNsense. `down` et `force_down` sont les pannes ; `loss` et `delay`
sont les avertissements de dpinger, et la passerelle répond toujours.

Le même appel renvoie délai, perte et déviation standard comme
**chaînes avec leur unité** (`"8.4 ms"`, `"0.0 %"`), et comme `"~"` quand
la passerelle n'a pas d'adresse de surveillance. `"~"` produit une série
de moins, jamais un zéro : une passerelle que personne ne ping n'a pas
zéro milliseconde de latence, elle n'en a aucune.

## La vérification de mise à jour n'est jamais déclenchée

`GET /api/core/firmware/status` renvoie le résultat de la *dernière*
vérification du pare-feu — celle que le propre tableau de bord d'OPNsense
montre. DumbMonit n'envoie jamais la forme `POST` de cet appel, ni
`/api/core/firmware/check`, qui enverrait le pare-feu vers le miroir à
chaque mesure, depuis chaque installation de DumbMonit.

Un pare-feu qui n'a jamais vérifié — ou qui vient d'installer une mise à
jour, qui vide le cache — répond `"status": "none"`, exactement comme un à
jour. DumbMonit distingue les deux, et jusqu'à ce qu'une vérification ait
tourné il publie `firmware_checked = 0` et rien d'autre : pas de
« zéro mise à jour en attente », pas de « aucun redémarrage nécessaire ».
Lancez une vérification une fois depuis System → Firmware, ou laissez le
propre calendrier du pare-feu le faire.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_opnsense_`.

| Famille | Métriques | Labels |
|---|---|---|
| Identité | `version_info` (les labels portent la version OPNsense, la version FreeBSD et la gamme produit) | |
| Machine | `uptime_seconds`, `load1/5/15`, `cpu_count`, `memory_used/total_bytes`, `memory_used_percent` (la mémoire inclut le cache ZFS, que FreeBSD compte comme utilisé), `swap_used/total_bytes`, `swap_used_percent` (absent sur une machine sans swap), `mbuf_used` et `mbuf_total` (clusters de buffers réseau utilisés par rapport à `kern.ipc.nmbclusters`), `mbuf_used_percent`, `mbuf_failures` (allocations refusées depuis le démarrage, un compteur) | |
| Stockage | `disk_used_bytes`, `disk_total_bytes`, `disk_used_percent` | `device`, `mountpoint` |
| Capteurs | `temperature_celsius` | `sensor` |
| Passerelles | `gateway_up` (0 seulement sur `down` et `force_down`), `gateway_monitored`, `gateway_delay_seconds`, `gateway_stddev_seconds`, `gateway_loss_percent`, `gateways_total`, `gateways_down` | `gateway`, `address` |
| Interfaces | `interface_up`, `interface_bytes_in/out`, `interface_packets_in/out`, `interface_errors_in/out`, `interface_drops`, `interface_collisions` (compteurs, stockés bruts), `interface_address_info` (l'adresse du moment, dans un label) | `interface`, `device`, et `address` pour le dernier |
| Pare-feu | `pf_states`, `pf_state_limit`, `pf_states_used_percent` | |
| DHCP | `dhcp_leases`, `dhcp_leases_active` | `backend` (`kea`, `isc`, `dnsmasq`) |
| VPN | `vpn_tunnel_up`, `vpn_peers_total`, `vpn_peers_connected` (WireGuard : handshake dans les trois minutes), `vpn_handshake_age_seconds` (WireGuard, le pair le plus récent ; absent tant qu'aucun pair ne s'est jamais connecté), `vpn_bytes_in/out`, `vpn_tunnels_down` | `kind` (`wireguard`, `openvpn`, `ipsec`), `tunnel` (le périphérique WireGuard, `wg0`, ou la description IPsec et OpenVPN) |
| Résolveur | `unbound_running`, `unbound_queries` (un compteur), `unbound_cache_hit_percent` | |
| Services | `service_running` (1 ou 0, une série par service que le pare-feu liste) | `service` |
| CARP | `carp_enabled`, `carp_maintenance_mode`, `carp_vip_master` (tous absents sur un pare-feu qui ne fait pas partie d'une paire) | `interface`, `vhid`, `address` |
| Firmware | `firmware_checked`, puis — seulement une fois qu'une vérification a tourné — `firmware_updates_pending`, `firmware_upgrade_available`, `firmware_reboot_required`, `firmware_connection_ok` | |
| Collection | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Les compteurs d'interface sont stockés bruts, comme des compteurs :
enveloppez-les dans `rate()` pour obtenir un débit. Faire ainsi signifie
qu'un redémarrage de pare-feu se montre comme un trou, pas comme un pic
de plusieurs gigaoctets par seconde.

`gateway_up` est 1 pour une passerelle qui n'est pas surveillée du tout,
parce qu'il n'y a rien à dire autrement ; `gateway_monitored` est ce qui
distingue les deux. Une passerelle sans adresse de surveillance ne produit
ni série de délai ni série de perte.

Un pare-feu qui ne fait pas partie d'une paire haute disponibilité n'a pas
d'adresse virtuelle CARP, et aucune série CARP du tout. C'est un pare-feu
sans cluster, pas un cluster dégradé : aucune règle ne peut se déclencher.

WireGuard, OpenVPN, IPsec, Unbound et Kea sont des plugins. Un pare-feu
qui n'en a pas un répond `404`, qui n'est ni une erreur de collection ni
une erreur d'authentification : une métrique de moins, et rien dans
l'interface.

Règles intégrées qui s'appliquent : Surveillance inaccessible, passerelle
Internet en panne, passerelle qui perd des paquets, passerelle lente,
table d'états du pare-feu qui se remplit, buffers réseau du pare-feu
épuisés, tunnel VPN en panne, résolveur du pare-feu arrêté, service
principal du pare-feu arrêté, pare-feu trop chaud, pare-feu laissé en
mode maintenance CARP, redémarrage du pare-feu en attente, mises à jour du
pare-feu en attente. Les notifications nomment la passerelle, le tunnel,
le service ou le capteur concerné.

## Ce qu'il faut préparer dans OPNsense

Les étapes ci-dessous sont celles que montre l'avis à côté du formulaire.
Le principe : un groupe avec seulement les privilèges de statut dont
DumbMonit a besoin, un utilisateur qui ne se connecte jamais, et une clé
d'API qui n'appartient à aucun de vos propres comptes. Ce groupe exact a
été vérifié contre chaque appel que DumbMonit fait, sur OPNsense 26.1.

1. System → Access → Groups → Add : nommez le groupe comme suit.

    ```
    dumbmonit
    ```

2. Dans ce groupe, sous Assigned Privileges, ajoutez les privilèges listés ci-dessous.

3. System → Access → Users → Add : nommez l'utilisateur comme suit, générez un mot de passe et faites-en un membre de ce groupe.

    ```
    dumbmonit
    ```

4. Modifiez l'utilisateur, cliquez sur + sous API keys et conservez le fichier téléchargé : il contient la clé et le secret, affichés une seule fois.

5. Dans DumbMonit, saisissez l'adresse du pare-feu (192.168.1.1 ou fw.lan:8443), la clé comme clé d'API et le secret comme secret d'API.

### Droits requis

- **Lobby: Dashboard** — Système, mémoire, disques, table d'états (obligatoire)
- **System: Gateways** — État, délai et perte des passerelles
- **Status: Interfaces** — État du lien et compteurs
- **Status: Services** — Services arrêtés
- **System: Firmware** — Mises à jour en attente et redémarrage
- **Interfaces: Virtual IPs: Status** — État CARP
- **Services: Unbound (MVC)** — Résolveur
- **Services: DHCP: Kea(v4)** — Baux, si vous utilisez Kea
- **Services: Dnsmasq DNS/DHCP: Settings** — Baux, si vous utilisez Dnsmasq
- **VPN: WireGuard: Status** — Tunnels WireGuard
- **Status: OpenVPN** — Tunnels OpenVPN
- **Status: IPsec** — Tunnels IPsec
- **Diagnostics: Logs: Firewall: Live View** — Uniquement pour l'option carte du pare-feu en direct

!!! warning
    Une clé d'API est un mot de passe qui n'expire jamais : limitez les réseaux sources du groupe à l'adresse de l'hôte DumbMonit et n'accordez jamais All pages. OPNsense livre un certificat auto-signé : cochez « Accept an unverifiable certificate » si la connexion est refusée.

Documentation de l'éditeur : https://docs.opnsense.org/development/how-tos/api.html

### Pourquoi ces droits

Un privilège omis ne coûte que les métriques qu'il porte, rien d'autre ;
seul `Lobby: Dashboard` est obligatoire. OPNsense n'a pas de variante en lecture
seule pour certains d'entre eux : `Status: Services` permet aussi de démarrer et
d'arrêter un service, `System: Firmware` de lancer une mise à jour, et les
privilèges Unbound, Kea et Dnsmasq couvrent aussi leurs réglages. DumbMonit ne
fait que lire, mais pour limiter ce qu'une clé volée permettrait, limitez les
réseaux sources du groupe à l'adresse de l'hôte DumbMonit, et n'accordez jamais
*All pages*.

On ne demande jamais au pare-feu de chercher des mises à jour : DumbMonit lit le
résultat de la vérification qu'OPNsense fait de lui-même, ou que vous lancez
depuis System → Firmware.

Adresse : `192.168.1.1`, `fw.lan`, `fw.lan:8443`, `[fd00::1]` ou une URL
complète. Le schéma `https` et le port 443 sont ajoutés s'ils manquent.

Certains privilèges couvrent plus d'un appel. `Lobby: Dashboard` porte les
points de terminaison d'information système, horloge, mémoire, disque,
swap, buffer réseau et température ainsi que la table d'états ;
`Status: Interfaces` porte la vue d'ensemble des interfaces avec ses
compteurs et adresses. Un privilège laissé de côté coûte exactement les
métriques qu'il porte : la sonde réussit quand même, et `scrape_errors`
reste à zéro, parce qu'un `403` sur un appel optionnel est lu comme
« ce compte ne peut pas voir ça », pas comme un échec. Seul
`Lobby: Dashboard` est requis : sans lui la sonde ne peut même pas lire le
nom et la version du pare-feu, et rapporte une erreur d'authentification.

### camelCase ou snake_case

OPNsense 25.7 a renommé ses chemins d'API de `systemResources` à
`system_resources`. Les deux orthographes répondent encore, mais les
privilèges comparent le chemin lettre par lettre : sur 25.7 et plus récent
une clé restreinte obtient `403` sur l'ancienne orthographe, et sur 25.1
sur certaines des nouvelles. DumbMonit demande d'abord l'orthographe
actuelle et se replie sur l'autre en cas de `403`, le même groupe
fonctionne donc des deux côtés du renommage.

## Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `scheme` | Protocole | `https` | HTTPS convient à un pare-feu dès le départ. HTTP seulement si l'interface web est servie en texte clair. |
| `port` | Port de l'interface web | `443` | Utilisé si l'adresse ne donne pas de port. |
| `insecure_tls` | Accepter un certificat non vérifiable | `false` | OPNsense livre un certificat autosigné par défaut. |
| `request_timeout_seconds` | Délai par requête (secondes) | `15` | Temps accordé à chaque appel d'API, de 1 à 120. |
| `gateways` | Surveiller les passerelles | `true` | Lit ce que dpinger dit de chaque passerelle : active ou en panne, délai d'aller-retour et perte de paquets. |
| `interfaces` | Surveiller les interfaces | `true` | Lit l'état de lien, les adresses et les compteurs d'octets, paquets, erreurs et pertes de chaque interface. |
| `firewall` | Surveiller la table d'états | `true` | Lit combien de connexions pf suit et la limite configurée. |
| `dhcp` | Compter les baux DHCP | `true` | Compte les baux que le pare-feu distribue, quel que soit le serveur qu'il fait tourner. Compteurs seulement. |
| `vpn` | Surveiller les tunnels VPN | `true` | Lit WireGuard, OpenVPN et IPsec : quels tunnels existent et lesquels sont actifs. |
| `unbound` | Surveiller le résolveur | `true` | Lit si Unbound répond. |
| `services` | Surveiller les services | `true` | Lit la liste des services que le pare-feu gère et lesquels tournent. |
| `carp` | Surveiller CARP | `true` | Lit les adresses virtuelles d'une paire haute disponibilité, et le mode maintenance persistant. |
| `firmware` | Surveiller les mises à jour | `true` | Lit le résultat de la dernière vérification de mise à jour du pare-feu et si un redémarrage est en attente. |
| `temperature` | Lire les capteurs de température | `true` | Lit les capteurs CPU et carte que le pare-feu expose. |
| `live_map` | Carte en direct du pare-feu | `false` | Montre d'où viennent, par pays, les connexions que vos règles bloquent ou laissent passer. Demande `Diagnostics: Logs: Firewall: Live View`. |

## Carte en direct

Avec l'option `live_map`, la page du moniteur gagne un panneau **Carte en
direct** : d'où viennent les connexions que vos règles bloquent ou laissent
passer, sur une carte du monde, sur les quinze dernières minutes — un disque par
pays (en pointillés quand l'essentiel est bloqué), de courts arcs qui filent
vers le pare-feu à mesure que les connexions arrivent, et les pays, règles et
ports les plus fréquents.

- **En lecture seule, à la demande.** Le serveur lit
  `GET /api/diagnostics/firewall/log` toutes les cinq secondes, seulement tant
  que quelqu'un a le panneau ouvert, et s'arrête une minute après. Le navigateur
  ne parle jamais au pare-feu. Donnez au groupe de l'utilisateur d'API le droit
  **Diagnostics: Logs: Firewall: Live View** (lecture seule).
- **Aucune adresse ne quitte le serveur.** Chaque ligne du journal est aussitôt
  réduite à son pays, son port, son protocole, sa règle et son action ; les
  adresses ne sont ni stockées ni envoyées au navigateur. Quinze minutes restent
  en mémoire (20 000 évènements au plus), rien n'est écrit en base ni dans
  VictoriaMetrics.
- **Les pays viennent d'une base locale.** Le serveur télécharge la base
  gratuite [IP to Country Lite](https://db-ip.com/db/download/ip-to-country-lite)
  de DB-IP (CC BY 4.0, sans compte) la première fois qu'une carte en direct en a
  besoin, la range dans `/data/geoip/` et la remplace quand elle a plus d'un
  mois. Les recherches sont locales : aucune adresse n'est envoyée à un service.
  Les plages privées, de bouclage, de lien local et CGNAT sont regroupées en
  **Réseau local**. Le panneau cite *IP Geolocation by DB-IP*, comme le demande
  la licence. La base se désactive dans **Réglages → Surveillance →
  Emplacements → Géolocalisation IP** : plus rien n'est téléchargé, et les pays
  apparaissent comme inconnus.
- **Les arcs ont besoin d'un emplacement.** Placez le pare-feu sur la
  [carte du monde](../using/world-map.md) (à la main, par son site, ou avec le
  site principal) ; sans emplacement, les disques sont tout de même dessinés.
- **Animation.** Quand le système demande de *réduire les animations*, les arcs
  qui filent sont omis ; les disques et les arcs fixes des pays les plus actifs
  restent.
- Un pare-feu interrogé par un agent relais n'a pas de carte en direct : le
  serveur ne joint pas son journal.

## Erreurs courantes

| Symptôme | Cause probable |
|---|---|
| Erreur d'authentification | La clé et le secret sont inversés, ou la clé a été supprimée de l'utilisateur. OPNsense ne montre le secret qu'une fois : s'il a été perdu, créez une autre clé et supprimez l'ancienne. |
| Une page web au lieu de JSON | La clé d'API est refusée, ou le compte ne peut pas atteindre ce point de terminaison. OPNsense sert alors sa page de connexion avec un `200` ; DumbMonit le rapporte comme un problème d'authentification plutôt qu'une réponse illisible. |
| Erreur de certificat | Certificat autosigné : installez-en un de confiance (ACME est un plugin) ou activez `insecure_tls`. |
| Aucune métrique de passerelle du tout | L'option `gateways` est désactivée, ou le groupe manque `System: Gateways`. Un pare-feu sans passerelle définie n'en a vraiment aucune. |
| Une passerelle ne montre ni délai ni perte | Elle n'a pas d'adresse de surveillance, donc dpinger ne la ping pas. OPNsense écrit `~` et DumbMonit ne publie aucune série plutôt qu'un zéro. |
| « Gateway down » se déclenche pour un lien qui fonctionne | L'adresse de surveillance est inaccessible même si le lien est actif — un cas courant avec un FAI qui abandonne l'ICMP. Réglez une autre adresse de surveillance dans System → Gateways. |
| Aucune métrique de table d'états | Le groupe manque `Lobby: Dashboard`. |
| Erreur d'authentification sur une clé qui fonctionne dans un outil navigateur | L'outil a utilisé l'autre orthographe du chemin. Avec une clé restreinte, OPNsense 25.7 et plus récent répondent `403` à `systemInformation` et `200` à `system_information`. |
| Aucune métrique de mise à jour sauf `firmware_checked` | Le pare-feu n'a jamais vérifié les mises à jour, ou vient d'en installer une. Lancez une vérification depuis System → Firmware. |
| Aucune métrique VPN | Le plugin n'est pas installé, ou aucun tunnel n'est configuré. Une connexion IPsec désactivée n'est délibérément pas rapportée comme en panne. |
| « Firewall resolver stopped » sur un pare-feu qui résout bien | Unbound est installé mais éteint parce que le pare-feu résout avec autre chose. Désactivez cette règle. |
| La version de firmware semble obsolète | Le pare-feu n'a pas vérifié le miroir récemment. DumbMonit lit la dernière vérification, il n'en déclenche jamais. |
| `scrape_errors` au-dessus de zéro | Un appel optionnel a échoué pour une raison autre qu'un privilège manquant ou un plugin manquant — un délai dépassé, généralement. Les autres métriques ne sont pas affectées. |

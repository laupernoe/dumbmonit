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

1. Dans l'interface web : System → Access → Groups → Add. Nommez le groupe comme suit.

    ```
    dumbmonit
    ```

2. Cochez ces privilèges dans le groupe, exactement comme OPNsense les nomme : Lobby: Dashboard (système, mémoire, disques, températures, table d'états), System: Gateways, Status: Interfaces, Status: Services, System: Firmware, Interfaces: Virtual IPs: Status (CARP) et Services: Unbound (MVC). Puis un par fonctionnalité que vous utilisez : Services: DHCP: Kea(v4) ou Services: Dnsmasq DNS/DHCP: Settings pour les baux, VPN: WireGuard: Status, Status: OpenVPN, Status: IPsec. Un privilège que vous laissez de côté coûte exactement les métriques qu'il porte, rien d'autre.

3. System → Access → Users → Add. Nommez le compte comme suit, laissez OPNsense générer un mot de passe brouillé (ce compte ne se connecte jamais : il répond avec sa clé), et faites-en un membre du groupe dumbmonit.

    ```
    dumbmonit
    ```

4. Modifiez à nouveau cet utilisateur et, sous API keys, cliquez sur +. OPNsense télécharge un petit fichier texte avec deux lignes, key= et secret=, et ne montre le secret que cette fois. Copiez la clé dans le champ Clé d'API de DumbMonit et le secret dans son champ Secret d'API.

5. Dans DumbMonit, saisissez l'adresse du pare-feu, par exemple « 192.168.1.1 » ou « fw.lan:8443 ». Le pare-feu n'est jamais sollicité pour vérifier les mises à jour : DumbMonit lit le résultat de la vérification qu'OPNsense lance sur son propre calendrier, ou que vous démarrez depuis System → Firmware.

!!! warning

    Ne réutilisez pas le compte avec lequel vous vous connectez : une clé d'API est un mot de passe qui n'expire jamais. OPNsense n'a pas de variante en lecture seule pour certains privilèges ci-dessus : Status: Services permet aussi de démarrer et d'arrêter un service, System: Firmware permet aussi de démarrer une mise à jour, et les privilèges Unbound, Kea et Dnsmasq couvrent aussi leurs réglages. DumbMonit ne fait toujours que lire, mais pour limiter ce qu'une clé qui fuit pourrait faire, restreignez les réseaux source du groupe à l'adresse de l'hôte DumbMonit. N'accordez jamais All pages. OPNsense livre aussi un certificat autosigné : si la connexion est refusée pour cette raison, cochez « Accepter un certificat non vérifiable » dans les options.

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

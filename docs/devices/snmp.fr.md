# Appareil SNMP

Le type le plus universel : presque tout le matériel réseau parle SNMP.
Commutateurs, routeurs, NAS, onduleurs et imprimantes sont tous concernés.
SNMP v1, v2c et v3 sont supportés.

## Ce qu'il surveille

Le profil de collecte est détecté automatiquement d'après le `sysObjectID` de
l'appareil (et, pour les onduleurs, son `sysDescr`). Sept profils sont
fournis avec le produit ; un appareil peut en correspondre à plusieurs, et
les profils s'incluent les uns les autres.

| Profil | S'applique à | Métriques (préfixe `dumbmonit_`) |
|---|---|---|
| Système (SNMPv2-MIB) | Chaque appareil | `system_uptime_seconds`, `system_info` (nom, description, emplacement, contact comme labels) |
| Interfaces réseau (IF-MIB) | Chaque appareil, en repli | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out`, `if_discards_in/out`, `if_admin_status`, `if_oper_status`, `if_speed_bps`, par interface |
| Ressources hôte (HOST-RESOURCES-MIB) | Net-SNMP (Linux, BSD, macOS), l'agent SNMP Windows, Synology, QNAP, Solaris | `host_uptime_seconds`, `host_processes`, `host_users`, `cpu_load_percent`, `storage_bytes_total/used` par système de fichiers, `memory_bytes_total/used`, `load_average` (1, 5 et 15 minutes, Net-SNMP seulement) |
| Onduleur (UPS-MIB) | APC, Eaton, MGE, CyberPower, Riello, Socomec, Tripp Lite, Vertiv, Salicru, et tout agent annonçant UPS-MIB | `ups_battery_status`, `ups_battery_seconds_on`, `ups_battery_minutes_remaining`, `ups_battery_charge_percent`, volts/ampères/température de la batterie, volts/hertz/défauts de ligne d'entrée, `ups_output_source`, volts/hertz/watts de sortie, `ups_output_load_percent`, `ups_alarms_present` |
| Imprimante (PRINTER-MIB) | HP, Kyocera, Epson, Canon, Brother, Lexmark, Ricoh, Xerox, Samsung, OKI, Konica Minolta | `printer_supply_level/max` par consommable, `printer_input_level/max` par bac, `printer_pages_printed`, `printer_status` |
| Commutateur Ethernet (EtherLike-MIB, LLDP) | Choisi à la main pour n'importe quel commutateur ; inclus par les profils de commutateur des constructeurs | Tout d'IF-MIB, plus par port `ether_fcs_errors`, `ether_alignment_errors`, `ether_symbol_errors`, `ether_late_collisions`, `ether_excessive_collisions`, `ether_mac_transmit_errors`, `ether_mac_receive_errors`, `ether_duplex_status` (1 inconnu, 2 semi, 3 complet), `if_broadcast_packets_in`, `if_multicast_packets_in` (64 bits), et `lldp_neighbor_info` (qui est branché où : `remote_sysname`, `remote_port`, `remote_port_desc` ; le numéro de port local est la deuxième partie du label `index`) |
| Commutateur Zyxel | Tout `sysObjectID` Zyxel (`1.3.6.1.4.1.890`) | Tout du commutateur Ethernet, plus `cpu_load_percent` (moyenne sur une minute), `zyxel_cpu_5min_percent`, `memory_usage_percent`, `zyxel_firmware_info` (firmware et modèle comme labels), et sur les commutateurs qui publient le moniteur matériel, `zyxel_temperature_celsius` et `zyxel_temperature_high_threshold_celsius` par capteur, `zyxel_fan_rpm` et `zyxel_fan_low_threshold_rpm` par ventilateur |

Les compteurs (octets d'interface, pages imprimées) sont stockés bruts :
utilisez `rate()` dans une requête pour obtenir un débit. Un redémarrage
d'appareil se montre comme une remise à zéro de compteur, jamais comme un pic
factice.

Les [règles intégrées](../alerting/rules.md) qui s'appliquent aux appareils
SNMP : Appareil inaccessible, CPU élevé, Disque presque plein, Système de
fichiers presque plein (prévision), Onduleur sur batterie, Batterie
d'onduleur faible, Port accumulant des erreurs, Port instable, CPU inhabituel
(référence).

### Commutateurs Zyxel

La partie standard du profil Zyxel (interfaces, erreurs Ethernet, LLDP) est
commune à chaque commutateur Zyxel. Les chiffres de CPU, mémoire et firmware
viennent de la branche privée que les commutateurs managés actuels partagent
sous `1.3.6.1.4.1.890.1.15` (dont les XS1930 et GS1900) ; les modèles plus
anciens et les pare-feu Zyxel organisent les leurs différemment, et ces
séries restent simplement vides là. Le profil a été vérifié sur un vrai
XS1930-10 ; ses séries de température et de ventilateur suivent la MIB de
moniteur matériel publiée par Zyxel mais n'ont pas encore été observées sur
du matériel réel.

`dot3StatsFrameTooLongs` n'est délibérément pas lu : sur le XS1930 elle
compte des milliards de trames jumbo parfaitement saines, et une série qui
monte toujours sur un réseau sain apprend seulement aux gens à l'ignorer.

### Contrôleurs de gestion de serveur (BMC) {#server-management-controllers-bmc}

Le propre agent SNMP d'un BMC décrit le BMC, pas le serveur : son petit Linux
ARM, sa mémoire, ses liens réseau. Un BMC Supermicro, par exemple, répond
comme Net-SNMP et obtient le profil Ressources hôte, qui lit tout ce que cet
agent publie sur lui-même. Les ventilateurs, températures, alimentations et
disques du serveur sont lus via [Redfish](redfish.md) à la place.

## Ce qu'il faut préparer sur l'appareil

1. Ouvrez l'interface d'administration de l'appareil.
2. Cherchez la section SNMP, souvent sous « Network », « Services » ou
   « Administration ».
3. Activez SNMP v2c et notez la communauté en lecture seule (« public » par
   défaut sur beaucoup d'appareils).
4. Si l'appareil filtre par adresse, autorisez l'adresse du serveur
   DumbMonit.
5. Revenez dans DumbMonit, saisissez l'adresse et la communauté : le reste est
   détecté automatiquement.

!!! warning
    Une communauté n'est pas chiffrée sur le réseau. Sur un réseau partagé,
    préférez SNMP v3, qui authentifie et chiffre.

## Identifiants

| Identifiant | Champs |
|---|---|
| SNMP v1 / v2c (communauté) | La communauté en lecture seule. |
| SNMP v3 | Nom d'utilisateur ; authentification optionnelle (MD5, SHA-1, SHA-224, SHA-256, SHA-384 ou SHA-512 et une phrase de passe) ; confidentialité optionnelle (DES, AES-128, AES-192 ou AES-256 et une phrase de passe) ; nom de contexte optionnel. |

Adresse : une IP ou un nom d'hôte, par exemple `192.168.1.10`. Le port par
défaut est 161 ; écrivez `host:port` pour en utiliser un autre.

## Options

Ce type ne lit aucune option. Les étiquettes libres restent disponibles et
sont copiées sur chaque série comme `tag_<key>`.

## Erreurs courantes

| Symptôme | Cause probable |
|---|---|
| *Inaccessible* juste après l'ajout | Mauvaise communauté, SNMP non activé, ou l'appareil n'accepte que les requêtes depuis des adresses autorisées. L'appareil ne répond jamais à une mauvaise communauté : cela ressemble exactement à un appareil éteint. |
| Seulement des séries `system_*` et `if_*` | Le `sysObjectID` n'a correspondu à aucun profil spécifique. IF-MIB et System s'appliquent à tout ; HOST-RESOURCES, UPS, PRINTER et Zyxel ont besoin d'un OID constructeur connu. Sur un commutateur d'une autre marque, choisissez le profil Commutateur Ethernet à la main. |
| Pas de trafic d'interface | L'appareil n'expose pas les compteurs 64 bits `ifXTable`. |
| Scan vide | Le scan essaie une communauté sur chaque adresse ; les appareils avec une autre communauté ou SNMP v3 seulement ne répondent pas. |

Utilisez **Sonder maintenant** sur la page de l'appareil : il rapporte le
nombre d'échantillons et les noms de série produits par une sonde.

# Sauvegarde des configurations réseau

Gardez chaque version de la configuration d'un commutateur, d'un routeur ou
d'un pare-feu, voyez ce qui a changé ligne par ligne, et soyez prévenu quand
elle change, à la manière d'Oxidized ou de RANCID. DumbMonit lit la
configuration par SSH (ou par l'API de l'équipement pour OPNsense et PAN-OS),
ne garde une nouvelle version que si la configuration a réellement changé, et
range chaque version compressée et chiffrée.

## Équipements pris en charge {#supported-devices}

Chaque modèle d'équipement est un **profil** livré avec DumbMonit. Un profil
dit comment reconnaître l'invite, couper la pagination, passer en mode
privilégié et quelles commandes envoyer. Rien de ce qui est saisi dans
l'interface n'est jamais envoyé à l'équipement : seulement les commandes du
profil choisi, telles qu'elles sont écrites.

| Modèle | Profil | Par | Commandes |
|---|---|---|---|
| Cisco IOS / IOS-XE | `cisco_ios` | terminal SSH, `enable` si besoin | `terminal length 0`, `terminal width 0`, `show running-config` |
| Cisco NX-OS | `cisco_nxos` | terminal SSH | `terminal length 0`, `terminal width 511`, `show running-config` |
| Commutateurs Aruba / HPE (ArubaOS-Switch, ProCurve, AOS-CX) | `aruba_hpe` | terminal SSH, `enable` si besoin | `no page`, `show running-config` |
| MikroTik RouterOS | `mikrotik_routeros` | commande SSH | `/export terse` |
| Juniper Junos | `juniper_junos` | commande SSH | `show configuration \| display omit \| no-more` |
| Fortinet FortiGate (FortiOS) | `fortigate` | terminal SSH, pagination répondue | `show` |
| pfSense | `pfsense` | commande SSH | `cat /cf/conf/config.xml` |
| OPNsense | `opnsense` | commande SSH | `cat /conf/config.xml` |
| OPNsense | `opnsense_api` | API HTTPS | `GET /api/core/backup/download/this` |
| Ubiquiti EdgeOS / UniFi USG | `ubiquiti_edgeos` | terminal SSH | `terminal length 0`, `show configuration commands` |
| Commutateur / point d'accès UniFi | `unifi_device` | commande SSH | `mca-ctrl -t dump-cfg` |
| Stormshield SNS | `stormshield` | commande SSH | les fichiers de `/usr/Firewall/ConfigFiles`, lus l'un après l'autre |
| Palo Alto PAN-OS | `paloalto_api` | API XML HTTPS | `show config running` |

Les profils sont dans `profiles/netconfig/*.yaml` dans les sources. Outre les
commandes, chacun liste les **lignes volatiles** de son modèle (horodatage du
dernier changement, période d'horloge, valeurs rechiffrées à chaque lecture)
et les **secrets** qu'il masque (mots de passe, communautés SNMP, clés
partagées, clés privées).

## Ports réseau à ouvrir {#network-ports-to-open}

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 22 | TCP | SSH (port par défaut), pour tous les profils sauf ceux par API |
| 443 | TCP | Seulement avec un profil par API (API OPNsense, PAN-OS) |

Un équipement que le serveur ne joint pas peut être sauvegardé par un **agent
relais** de son réseau, comme tout autre équipement : l'agent se connecte,
renvoie la configuration au serveur et n'en garde rien.

## Sauvegarder la configuration d'un équipement {#back-up-a-device-configuration}

1. Choisissez le modèle dans « Device model » : il décide des commandes envoyées, et rien d'autre n'est jamais exécuté sur l'équipement.
2. Créez sur l'équipement un compte en lecture seule qui peut se connecter en SSH et afficher la configuration courante.
3. Pour OPNsense par son API, créez une clé d'API pour un utilisateur autorisé à télécharger les sauvegardes ; pour PAN-OS, générez la clé d'API d'un rôle en lecture seule.
4. La première connexion SSH mémorise la clé d'hôte de l'équipement : si elle change un jour, les sauvegardes s'arrêtent et une alerte vous demande de vérifier.

Ce dont le compte a besoin :

- Un compte en lecture seule — autorisé à se connecter en SSH et à afficher la configuration

Le compte SSH a un **mot de passe enable** facultatif, pour un profil qui
passe en mode privilégié (Cisco IOS, Aruba) avec un autre mot de passe que
celui du compte ; vide, c'est le mot de passe du compte qui sert. Un compte qui
arrive directement en mode privilégié (privilège 15) n'en a pas besoin. Les
authentifications par mot de passe et par clavier interactif sont prises en
charge ; les clés ne le sont pas.

!!! warning
    Les configurations contiennent des secrets : elles sont rangées chiffrées, affichées secrets masqués, et seul un administrateur connecté peut les révéler ou les télécharger.

## Déroulement d'une sauvegarde {#how-a-backup-works}

Toutes les **Back up every (hours)** (24 par défaut), le serveur lit la
configuration. Une lecture en échec est retentée au bout de 15 minutes.
**Sauvegarder maintenant**, sur la page de l'équipement, la lit tout de suite.

La configuration est ensuite réduite à sa forme comparable : les lignes
volatiles du profil sont écartées, ainsi que les espaces de fin de ligne. Si
cette forme est celle de la dernière version, rien de nouveau n'est rangé : la
dernière version est seulement marquée comme revue. Sinon, une nouvelle
version est gardée, avec le nombre de lignes ajoutées et retirées. La première
version sert de référence, jamais de changement.

Bornes : chaque étape (connexion, invite, commande) dispose de **Timeout per
step (seconds)** (60 par défaut, de 10 à 110), et la lecture entière du double.
Une configuration de plus de 4 Mio est refusée plutôt que rangée tronquée.

## Clé d'hôte SSH {#ssh-host-key}

La première connexion SSH fait confiance à la clé présentée et en mémorise
l'empreinte SHA-256, affichée sur la page de l'équipement. Ensuite, une clé
différente **coupe la connexion avant l'envoi de tout identifiant** : aucun mot
de passe ne part vers un équipement qui pourrait être un imposteur. La page de
l'équipement montre les deux empreintes, l'alerte **SSH host key changed** se
lève, et les sauvegardes s'arrêtent jusqu'à ce qu'un administrateur compare la
nouvelle empreinte à celle que montre l'équipement (`show ssh` /
`show crypto key mypubkey`, la console de l'appliance) et l'accepte. Un
équipement remplacé ou réinstallé change de clé ; une interception aussi.

## Versions et changements {#viewing-versions-and-changes}

La page de l'équipement liste les versions. En choisir une montre le diff
ligne à ligne avec la précédente (ou avec n'importe quelle autre) : chaque ligne
ajoutée commence par `+` et chaque ligne retirée par `−`, pour que le
changement ne se lise pas seulement à la couleur.

Les secrets sont **masqués** à l'affichage : chacun devient `<masked:xxxxxx>`,
où `xxxxxx` est une courte empreinte calculée avec le secret de cette instance.
Un mot de passe changé apparaît donc comme une ligne modifiée sans être révélé,
et l'empreinte ne peut être vérifiée contre une supposition nulle part
ailleurs. Le masquage suit les règles du profil, plus des règles génériques
pour tout ce qui s'appelle mot de passe, secret ou clé partagée.

Qui voit quoi :

| | Lecteur | Opérateur | Administrateur |
|---|---|---|---|
| Liste des versions, dates, lignes changées | oui | oui | oui |
| Contenu et diffs, secrets masqués | non | oui | oui |
| Secrets en clair, téléchargement | non | non | oui, connecté, inscrit au journal d'audit |

Un jeton d'API n'obtient jamais une configuration en clair, ni un
téléchargement.

## Stockage et rétention {#storage-and-retention}

Les versions sont gardées dans la base de DumbMonit, compressées (gzip) et
chiffrées avec la clé d'instance (AES-256-GCM, comme les identifiants des
équipements). L'empreinte de comparaison est un HMAC sous une clé dérivée du
secret d'instance, jamais une empreinte nue d'un texte qui contient des
secrets.

**Versions kept** (100 par défaut) et **Keep versions for (days)** (365 par
défaut ; 0 pour aucune limite d'âge, d'après la date où une version a été vue
pour la dernière fois) décident de ce qui est supprimé. La dernière version
reste toujours.

La sauvegarde de DumbMonit emporte les équipements, leurs options et
identifiants, et les clés d'hôte mémorisées. Les versions de configuration
elles-mêmes n'y sont incluses que si vous cochez **Include network
configuration versions** à l'export ; c'est désactivé par défaut.

## Alertes {#alerts}

| Règle | Se déclenche quand |
|---|---|
| Network configuration changed | La dernière sauvegarde a trouvé une configuration différente de la version précédente (`dumbmonit_netconfig_config_changed`). Elle se résout à la sauvegarde suivante sans changement. |
| SSH host key changed | L'équipement a présenté une autre clé d'hôte ; les sauvegardes sont arrêtées (`dumbmonit_netconfig_host_key_changed`). |

Un équipement injoignable lève **Monitor unreachable**, un compte refusé
s'affiche sur la page de l'équipement. Voir [Règles](../alerting/rules.md).

## Options {#options}

| Option | Défaut | Effet |
|---|---|---|
| Device model | `cisco_ios` | Le profil : comment parler à l'équipement et quelles commandes envoyer. |
| Back up every (hours) | 24 | De 1 à 168. |
| Versions kept | 100 | De 1 à 1000. |
| Keep versions for (days) | 365 | De 0 (aucune limite d'âge) à 3650. |
| Timeout per step (seconds) | 60 | De 10 à 110. |
| Accept an unverifiable certificate | désactivé | Profils par API seulement, pour un équipement à vous derrière un certificat auto-signé. |

# Agent Synology (DSM 7)

Un petit paquet qui exécute l'agent DumbMonit sur un NAS Synology. En plus du
processeur, de la mémoire, des disques et du réseau, il lit le NAS lui-même :
grappes RAID, volumes, températures des disques, paquets installés, services de
fichiers et onduleur (voir [Ce qui est surveillé](#ce-qui-est-surveille)). Les
tâches Hyper Backup, les mises à jour de DSM et le verdict SMART de DSM demandent
[Synology DSM](synology.md), qui passe par l'API DSM. Les deux peuvent tourner
sur le même NAS ; [Paquet ou API ?](synology.md#paquet-ou-api) les compare.

L'agent ne fait que des connexions sortantes vers votre serveur DumbMonit : le
paquet n'ouvre aucun port sur le NAS. Il tourne sous un utilisateur de paquet
dédié, pas en root.

## Installation

1. Dans DumbMonit, ouvrez **Ajouter un moniteur**, choisissez **Synology DSM**,
   puis **Paquet DumbMonit Agent**. La page crée un jeton d'enrôlement et affiche
   l'URL du serveur et l'empreinte du certificat que demande l'assistant.
   Gardez la page ouverte : le jeton y est affiché.
2. Téléchargez le `.spk` correspondant à votre NAS (`x86_64` ou `aarch64`) depuis
   la même page, ou depuis la page des versions de DumbMonit sur GitHub.
3. Dans DSM, ouvrez le **Centre de paquets**, cliquez sur **Installation
   manuelle** et choisissez le fichier. DSM avertit que l'éditeur est inconnu :
   le paquet n'est pas encore signé.
4. Dans l'assistant, saisissez l'URL du serveur (HTTPS uniquement, par exemple
   `https://192.168.1.10:8080`), collez le jeton, puis l'empreinte du certificat
   du serveur (pin, `sha256//...`) affichée dans **Réglages > Système > HTTPS**.
   L'empreinte ne peut rester vide que si le serveur a un certificat d'une
   autorité reconnue ; avec le certificat auto-signé par défaut, l'installation
   échoue tant qu'elle n'est pas collée.
5. Terminez. Le NAS apparaît dans DumbMonit en moins d'une minute. Si le serveur
   est injoignable, que l'empreinte ne correspond pas ou que le jeton est
   refusé, l'installation s'arrête avec un message : corrigez l'adresse ou
   l'empreinte, ou créez un nouveau jeton et recommencez.

L'agent vérifie toujours ses propres mises à jour avec la clé de signature de
DumbMonit.

## Ce qui est surveillé {#ce-qui-est-surveille}

L'agent reconnaît DSM à `/etc.defaults/VERSION` et `/usr/syno`, puis lit, sans
root :

| Quoi | Source | Affiché comme |
|---|---|---|
| Modèle et version de DSM | `/etc.defaults/VERSION`, `synoinfo.conf` (`upnpmodelname`) | En-tête du panneau NAS, parc NAS |
| Grappes RAID (données, système, échange, cache SSD) | `/proc/mdstat` : membre manquant ou en échec, reconstruction, resynchronisation, vérification | Grappes RAID, alerte **RAID du NAS dégradé** |
| Volumes | `/proc/mounts` (`/volumeN`) et l'occupation disque habituelle | Volumes, alertes **Volume du NAS en lecture seule** et disque presque plein |
| Disques | `/sys/block` (modèle, taille, HDD/SSD/NVMe) ; température, durée de vie restante des SSD et secteurs illisibles depuis `/run/synostorage/disks` quand DSM les expose | Disques, alertes **Température de disque du NAS élevée**, **Secteurs illisibles en hausse sur le NAS**, **SSD du NAS en fin de vie** |
| Paquets (Drive, Photos, Hyper Backup, Active Backup, Container Manager…) | `/var/packages/*/INFO` et le témoin `enabled` ; état réel par `synopkg status` quand DSM le permet à l'utilisateur du paquet | Paquets, alerte **Paquet du NAS arrêté** |
| SMB, NFS, SSH | Leur processus (`smbd`, `nfsd`, `sshd`) | Actif ou Arrêté dans le panneau, sans alerte |
| Onduleur | `upsc`, quand le serveur d'onduleur de DSM tourne (onduleur en USB, ou serveur réseau activé) | Onduleur dans le panneau, alertes [Onduleur via NUT](../alerting/rules.md#ups-behind-nut) |
| Température du processeur | `/sys/class/hwmon` | Température dans le panneau |

Le numéro de série n'est jamais lu. Une source absente ou illisible est sautée :
rien n'est inventé, et aucune alerte ne part sur ce qui n'a pas été lu. La santé
du stockage (0 sain, 1 attention, 2 critique) alimente la vue du parc NAS, comme
le moniteur par l'API DSM.

### Ce qui demande plus que l'utilisateur du paquet

Le paquet tourne sous son propre utilisateur (`conf/privilege` : `run-as:
package`) et ne demande rien de plus. Sans root :

- Le **SMART** des disques (`smartctl`) ne peut pas ouvrir les périphériques :
  pas de verdict SMART, seulement la température et l'usure que DSM publie dans
  `/run/synostorage`.
- **État des paquets** : sur certaines versions de DSM, `synopkg status` est
  refusé à l'utilisateur du paquet. Le panneau montre alors seulement si chaque
  paquet est activé, et l'alerte **Paquet du NAS arrêté** reste muette.
- **Hyper Backup**, **mises à jour de DSM et des paquets**, **connexions SMB** :
  seule l'API DSM les rapporte. Ajoutez un moniteur [Synology DSM](synology.md)
  avec un compte en lecture seule à côté du paquet.

## Désinstallation

Dans le **Centre de paquets**, ouvrez **DumbMonit Agent**, choisissez
**Désinstaller**. La configuration et le jeton sont supprimés avec lui. Puis
supprimez l'équipement dans DumbMonit si vous ne voulez plus le voir listé.

## Limites

- DSM 7.0 ou plus récent uniquement. DSM 6 n'est pas pris en charge.
- Pas de résultats Hyper Backup ni Active Backup, pas de mises à jour de DSM ni
  des paquets, pas de verdict SMART de DSM, pas de connexions actives :
  l'utilisateur du paquet ne peut pas les lire. Passez par l'[API DSM](synology.md).
- L'assistant ne demande pas encore les réglages du canal chiffré de l'agent.

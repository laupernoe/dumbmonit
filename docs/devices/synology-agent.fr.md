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

## Avant de commencer

- Un NAS Synology sous **DSM 7.0 ou plus récent** (DSM 6 n'est pas pris en
  charge ; utilisez l'[API DSM](synology.md) dans ce cas).
- Vous connaissez l'architecture du NAS, `x86_64` ou `aarch64`. La page du
  paquet propose les deux téléchargements. En cas de doute, cherchez le modèle
  dans les caractéristiques Synology.
- Le NAS peut ouvrir une connexion HTTPS vers le serveur DumbMonit (en général
  `https://<serveur>:8080`). Rien n'a besoin de joindre le NAS.

## Installation {#install}

1. Dans DumbMonit, cliquez sur **Ajouter une surveillance**, choisissez un site
   (ou **Aucun site**), **Synology DSM**, puis **Utiliser le paquet** (*Paquet
   DumbMonit Agent*). Vous devez voir une page avec **Téléchargez le paquet pour
   votre NAS**, une **URL du serveur**, une **empreinte du certificat** et un
   jeton. Gardez la page ouverte : le jeton n'y est affiché qu'ici.
2. Cliquez sur le téléchargement de votre NAS (`x86_64` ou `aarch64`), ou prenez
   le `.spk` sur la page des versions de DumbMonit sur GitHub.
3. Dans DSM, ouvrez le **Centre de paquets**, cliquez sur **Installation
   manuelle** et choisissez le fichier. DSM avertit que l'éditeur est inconnu :
   le paquet n'est pas encore signé. Acceptez pour continuer.
4. L'assistant demande trois valeurs. Copiez chacune depuis la page DumbMonit :
    - **URL du serveur** : HTTPS uniquement, par exemple
      `https://192.168.1.10:8080`.
    - **Jeton d'enrôlement** : la valeur affichée sur la page.
    - **Empreinte du certificat** (`sha256//...`) : collez-la exactement comme
      affichée. Ne la laissez vide que si le serveur a un certificat d'une
      autorité reconnue. Avec le certificat autosigné par défaut, l'installation
      échoue sans elle.
5. Terminez. Le NAS apparaît dans DumbMonit en moins d'une minute, avec son
   modèle et sa version de DSM.

Si le serveur est injoignable, que l'empreinte ne correspond pas ou que le jeton
est refusé, l'installation s'arrête avec un message et rien ne reste installé.

### Si l'installation échoue

| Message ou symptôme | Que faire |
|---|---|
| L'assistant refuse l'URL | Elle doit commencer par `https://`. DumbMonit ne sert que du HTTPS. |
| L'installation s'arrête, serveur injoignable | Depuis le NAS, l'adresse du serveur doit répondre. Vérifiez l'adresse, le port (8080 par défaut) et tout pare-feu entre les deux. |
| L'installation s'arrête, l'empreinte ne correspond pas | Recopiez l'empreinte depuis la page d'ajout, sans espaces. Si vous avez remplacé le certificat du serveur, l'empreinte a changé. |
| L'installation s'arrête, jeton refusé | Le jeton a été utilisé, révoqué ou mal saisi. Revenez à la page d'ajout, créez-en un nouveau et recommencez. |
| Le paquet est installé mais le NAS n'apparaît jamais | Dans le **Centre de paquets**, vérifiez que **DumbMonit Agent** est démarré, puis patientez une minute. |

L'agent vérifie toujours ses propres mises à jour avec la clé de signature de
DumbMonit.

## Ce qui est surveillé {#ce-qui-est-surveille}

L'agent reconnaît DSM à `/etc.defaults/VERSION` et `/usr/syno`, puis lit, sans
root :

| Quoi | Source | Affiché comme |
|---|---|---|
| Modèle et version de DSM | `/etc.defaults/VERSION`, `synoinfo.conf` (`upnpmodelname`) | En-tête du panneau NAS |
| Grappes RAID (données, système, échange, cache SSD) | `/proc/mdstat` : membre manquant ou en échec, reconstruction, resynchronisation, vérification | Grappes RAID, alerte **RAID du NAS dégradé** |
| Volumes | `/proc/mounts` (`/volumeN`) et l'occupation disque habituelle | Volumes, alertes **Volume du NAS en lecture seule** et disque presque plein |
| Disques | `/sys/block` (modèle, taille, HDD/SSD/NVMe) ; température, durée de vie restante des SSD et secteurs illisibles depuis `/run/synostorage/disks` quand DSM les expose | Disques, alertes **Température de disque du NAS élevée**, **Secteurs illisibles en hausse sur le NAS**, **SSD du NAS en fin de vie** |
| Paquets (Drive, Photos, Hyper Backup, Active Backup, Container Manager…) | `/var/packages/*/INFO` et le témoin `enabled` ; état réel par `synopkg status` quand DSM le permet à l'utilisateur du paquet | Paquets, alerte **Paquet du NAS arrêté** |
| SMB, NFS, SSH | Leur processus (`smbd`, `nfsd`, `sshd`) | Actif ou Arrêté dans le panneau, sans alerte |
| Onduleur | `upsc`, quand le serveur d'onduleur de DSM tourne (onduleur en USB, ou serveur réseau activé) | Onduleur dans le panneau, alertes [Onduleur via NUT](../alerting/rules.md#ups-behind-nut) |
| Température du processeur | `/sys/class/hwmon` | Température dans le panneau |

Le numéro de série n'est jamais lu. Une source absente ou illisible est sautée :
rien n'est inventé, et aucune alerte ne part sur ce qui n'a pas été lu.

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

## Mise à jour

L'agent est l'agent DumbMonit habituel : il suit la version de votre serveur par
les [mises à jour automatiques](agent.md#automatic-updates) et vérifie chaque
mise à jour avec la clé de signature de DumbMonit. Vous n'installez pas un
nouveau `.spk` après chaque mise à jour de DumbMonit : le paquet se met à jour
tout seul avec le serveur. Le binaire mis à jour vit dans le dossier `var`
inscriptible du paquet, et c'est lui que le Centre de paquets démarre. Un NAS
qui a encore un paquet antérieur à 0.17.1 ne peut pas encore le faire :
installez une fois le `.spk` actuel par-dessus (la configuration est
conservée), il suit ensuite le serveur.

## Désinstallation

1. Dans le **Centre de paquets**, ouvrez **DumbMonit Agent**.
2. Choisissez **Désinstaller**. La configuration et le jeton sont supprimés avec
   lui.
3. Dans DumbMonit, supprimez la surveillance si vous ne voulez plus la voir
   listée. Jusque-là elle apparaît comme injoignable.

## Limites

- DSM 7.0 ou plus récent uniquement. DSM 6 n'est pas pris en charge.
- Pas de résultats Hyper Backup ni Active Backup, pas de mises à jour de DSM ni
  des paquets, pas de verdict SMART de DSM, pas de connexions actives :
  l'utilisateur du paquet ne peut pas les lire. Passez par l'[API DSM](synology.md).
- L'assistant ne demande pas encore les réglages du canal chiffré de l'agent.

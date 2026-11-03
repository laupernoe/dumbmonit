# Agent Synology (DSM 7)

Un petit paquet qui exécute l'agent DumbMonit sur un NAS Synology. Il remonte
le processeur, la mémoire, les disques et le réseau. Pour les volumes, pools de
stockage, santé des disques et tâches de sauvegarde, voir
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

## Désinstallation

Dans le **Centre de paquets**, ouvrez **DumbMonit Agent**, choisissez
**Désinstaller**. La configuration et le jeton sont supprimés avec lui. Puis
supprimez l'équipement dans DumbMonit si vous ne voulez plus le voir listé.

## Limites

- DSM 7.0 ou plus récent uniquement. DSM 6 n'est pas pris en charge.
- Uniquement ce que l'agent collecte sur n'importe quelle machine Linux ; pas de
  données de volumes, RAID ou sauvegardes propres à Synology avec ce paquet.
- L'assistant ne demande pas encore les réglages du canal chiffré de l'agent.

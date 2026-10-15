# Notifications push (PWA)

DumbMonit peut faire sonner votre téléphone ou votre ordinateur lui-même, à la
manière d'une application native, sans installer d'application ni créer de
compte où que ce soit : le propre service de push du navigateur (Google,
Mozilla, Apple, Microsoft) porte un message chiffré de bout en bout de votre
serveur vers l'appareil, et la notification s'affiche même avec tous les
onglets DumbMonit fermés. C'est le standard **Web Push** ; le site peut aussi
être installé comme une application (une *progressive web app*, PWA) pour
s'ouvrir depuis une icône comme n'importe quelle autre.

Deux étapes : activer le push sur chaque appareil (cette page), puis décider
quelles alertes y vont avec un canal **Web Push**
([Canaux de notification](../notifications.md#web-push)).

## Avant de commencer

- **HTTPS est requis.** Les navigateurs ne proposent le push qu'à une origine
  sécurisée. Mettez DumbMonit derrière un reverse proxy avec un certificat
  (Caddy, Traefik, Nginx Proxy Manager…) et ouvrez-le via cette adresse.
  `http://localhost` est aussi accepté, ce qui n'est utile que pour tester
  sur le serveur lui-même. En simple `http://192.168.x.x`, la section indique
  **Nécessite HTTPS**.
- **Le serveur a besoin de HTTPS sortant** vers les services de push :
  `fcm.googleapis.com` (Chrome, et Edge sur Android), `*.notify.windows.com`
  (Edge sur Windows), `updates.push.services.mozilla.com` (Firefox) et
  `*.push.apple.com` (Safari, iPhone, iPad). Rien n'entre : votre DumbMonit
  n'a pas besoin d'être accessible depuis l'internet, le téléphone n'a besoin
  de l'atteindre que quand vous touchez une notification.
- **Réglez `DUMBMONIT_PUBLIC_URL`** sur l'adresse HTTPS que vous utilisez :
  elle est donnée aux services de push comme contact de l'émetteur, et c'est
  l'adresse vers laquelle pointent les liens dans les autres canaux.

## Activer sur un téléphone ou un ordinateur

### Android, Windows, macOS, Linux

1. Ouvrez DumbMonit dans Chrome, Edge, Firefox ou Safari et connectez-vous.
2. Optionnel : installez-le comme une application (barre d'adresse →
   **Installer**, ou le menu du navigateur → **Ajouter à l'écran d'accueil**).
   Les notifications fonctionnent dans les deux cas.
3. **Réglages → Notifications push → Activer sur cet appareil**, et autorisez
   les notifications quand le navigateur le demande.
4. **Envoyer un test.** Une notification *DumbMonit — notification de test*
   devrait apparaître en quelques secondes.

### iPhone et iPad

Apple ne donne le push qu'aux sites ajoutés à l'écran d'accueil (iOS et
iPadOS 16.4 ou plus récent) :

1. Ouvrez DumbMonit dans **Safari**.
2. **Partager → Sur l'écran d'accueil**, puis ouvrez DumbMonit **depuis la
   nouvelle icône**.
3. Connectez-vous, puis **Réglages → Notifications push → Activer sur cet
   appareil**, et autorisez les notifications.
4. **Envoyer un test.**

Dans Safari sans installation, la section indique **Installer d'abord**.

## Choisir quelles alertes arrivent

Dans **Réglages → Notifications et rapports**, ajoutez un canal de type **Web Push**. Sans
réglage, chaque appareil abonné de chaque compte le reçoit ; listez des noms
de compte sous **Comptes** pour ne cibler que certaines personnes. La
politique habituelle du canal s'applique (gravité minimale, heures calmes,
filtre de routage…), vous pouvez donc par exemple garder un téléphone pour
les alertes critiques seulement.

Une notification lit `Critique · nas01` avec un court résumé ; la toucher
ouvre la page **Alertes**. La résolution d'une alerte remplace sa
notification plutôt que d'en ajouter une seconde.

## Appareils et comptes

Chaque compte voit et gère ses propres appareils : les lecteurs peuvent aussi
activer le push sur leur téléphone, cela ne change rien d'autre. La liste
montre le navigateur et le système, le service de push, quand l'appareil a
été ajouté et quand il a reçu un message pour la dernière fois ;
**Supprimer** l'oublie (et, sur l'appareil que vous utilisez, désabonne aussi
le navigateur). Un appareil que le service de push rapporte comme disparu
(l'application a été supprimée, les données du navigateur effacées) est
supprimé automatiquement au prochain envoi.

## Dépannage

| Ce que vous voyez | Quoi faire |
|---|---|
| **Nécessite HTTPS** | Ouvrez DumbMonit via son adresse HTTPS. |
| **Installer d'abord** (iPhone, iPad) | Partager → Sur l'écran d'accueil, ouvrez depuis l'icône. |
| **Bloqué** | Les notifications ont été refusées pour ce site : autorisez-les dans les réglages du site du navigateur (cadenas dans la barre d'adresse, ou iOS Réglages → Notifications → DumbMonit), puis rechargez. |
| **Non supporté** | Le navigateur n'a pas de Web Push (vieille version, certains navigateurs axés confidentialité) : utilisez un Chrome, Edge, Firefox ou Safari récent. |
| Le test dit *push service unreachable* | Le serveur ne peut pas atteindre le service de push : vérifiez son HTTPS sortant et son DNS. |
| Le test dit *refused the VAPID signature* | Généralement une horloge serveur incorrecte ; vérifiez le NTP sur l'hôte. |
| Rien n'arrive sur Android quand le téléphone dort | L'optimisation de batterie peut retarder le navigateur : exemptez-le, ou réglez l'**Urgence** du canal sur `high`. |

## Confidentialité et sécurité

- Les messages sont chiffrés pour chaque appareil (RFC 8291) avant de
  quitter le serveur : le service de push ne voit qu'un blob opaque,
  l'appareil le déchiffre.
- Le serveur signe chaque message avec sa propre paire de clés VAPID
  (RFC 8292), générée au premier démarrage et stockée chiffrée avec le
  secret de l'instance ; la clé privée n'est jamais renvoyée par l'API. Une
  [restauration](../install/backup.md) sur une nouvelle machine génère une
  nouvelle paire de clés : réactivez le push sur chaque appareil.
- L'adresse de l'abonnement vient du navigateur, le serveur n'accepte donc
  que des adresses HTTPS sur des IP publiques : les adresses privées,
  loopback et link-local sont refusées, à l'abonnement et à nouveau à chaque
  connexion.
- Le service worker (`/sw.js`) ne fait qu'afficher des notifications. Il ne
  met rien en cache et n'intercepte aucune requête : l'interface et l'API
  viennent toujours du serveur, jamais d'une copie périmée.

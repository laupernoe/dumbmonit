# Clés d'accès (passkeys)

Une clé d'accès permet de se connecter avec son empreinte, son visage, le code de
l'appareil ou une clé de sécurité (YubiKey…) au lieu de taper un mot de passe.
Elle remplace le mot de passe **et** le second facteur en un seul geste :
DumbMonit exige que l'appareil vous vérifie (biométrie ou code) et ne demande
jamais le code TOTP après une clé d'accès.

Les clés d'accès s'**ajoutent** au mot de passe : vous pouvez continuer à vous
connecter avec lui (et le TOTP) quand vous le voulez ou si aucune clé n'est à
portée.

## Prérequis

- **HTTPS.** Les navigateurs ne proposent les clés d'accès que sur des pages
  sécurisées : placez DumbMonit derrière un reverse proxy TLS.
  `http://localhost` est la seule exception (pratique pour essayer).
- **Un nom de domaine**, pas une adresse IP. Une clé d'accès est liée au domaine
  où elle a été créée (le *RP ID*, ici le nom d'hôte tapé dans le navigateur).
  Ouvrez `https://monit.example.org`, pas `https://192.168.1.10`.
- Renseignez `DUMBMONIT_PUBLIC_URL` (ou **Réglages → Notifications → URL
  publique**) avec cette adresse : DumbMonit refuse alors de créer des clés
  depuis une autre adresse, plutôt que de vous laisser enregistrer des clés qui
  ne marcheraient que là. Sans elle, l'adresse de la page où vous êtes est
  utilisée.
- Comptes locaux seulement. Les comptes qui se connectent par LDAP / Active
  Directory ou SSO gardent leur fournisseur, qui peut les révoquer.

Si vous changez de domaine, les clés existantes cessent de fonctionner (le
navigateur ne les propose plus) ; le mot de passe reste là pour vous connecter
et en ajouter de nouvelles.

## Ajouter une clé

**Réglages → Compte et sécurité → Clés d'accès → Ajouter une clé d'accès.**
Donnez-lui un nom (Portable, Téléphone, YubiKey), confirmez avec votre mot de
passe, puis suivez l'invite du navigateur. Jusqu'à 20 clés par compte : une par
appareil, et une clé de sécurité en secours.

Les clés conservées dans un gestionnaire de mots de passe ou le trousseau du
système (iCloud, Google) sont marquées **Synchronisée** : elles vous suivent d'un
appareil à l'autre.

## Se connecter

Sur la page de connexion, **Se connecter avec une clé d'accès** (visible dès
qu'une clé existe sur l'instance et que le navigateur les gère). Aucun nom
d'utilisateur à saisir : le navigateur liste vos clés pour ce site.

## Gérer

Dans la même section : **renommer** ou **supprimer** une clé. Supprimer la clé
d'un appareil perdu suffit à couper son accès. L'ajout, la suppression et
l'usage d'une clé figurent dans le journal d'audit de sécurité (administrateurs).

## En coulisses

- WebAuthn, algorithme ES256 (ECDSA P-256), attestation `none`, identifiants
  découvrables, vérification de l'utilisateur obligatoire.
- DumbMonit ne garde que la clé *publique* : une copie de la base ne connecte
  personne.
- Le compteur de signatures est contrôlé quand l'authentificateur en tient un (un
  compteur qui recule trahit une clé clonée) ; les clés synchronisées annoncent 0,
  ce qui est accepté.
- La vérification est faite par DumbMonit lui-même, sans OpenSSL : la crate
  habituelle `webauthn-rs` en dépend, ce que l'image statique musl n'autorise pas.
- Les clés d'accès ne font pas partie de la sauvegarde de l'instance (elles
  dépendent du domaine) : après une restauration sur une autre adresse,
  enregistrez-les de nouveau.

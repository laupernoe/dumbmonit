# SonicWall SonicOS

Un pare-feu SonicWall (TZ, NSa, NSsp, NSv) via l'API SonicOS 7 : le lien de
chaque interface rattachée à une zone. Volontairement minimal.

!!! warning "Bêta"

    Cette intégration a été construite à partir de ce que SonicWall publie sur
    l'API SonicOS, pas contre un appareil en fonctionnement. Les champs exacts
    de la réponse d'état des interfaces ne sont pas documentés : DumbMonit la
    lit avec tolérance — tout objet portant un `name` et une chaîne d'état
    (`status` ou `link_status`) est une interface. L'état des tunnels, de la
    HA et des licences n'est pas lu, faute de chemin d'API confirmé. Dites-nous
    ce qui casse.

DumbMonit se connecte en HTTP Basic (`Authorization: Basic`, jamais dans
l'URL) : `POST /api/sonicos/auth` ouvre la session, puis `GET
/api/sonicos/version` (optionnel) et `GET
/api/sonicos/reporting/interfaces/ipv4/status` sont lus, et `DELETE
/api/sonicos/auth` referme la session, même si une lecture a échoué. SonicOS
n'admet qu'une session d'API à la fois : la fermer compte.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | Interface de gestion (HTTPS, par défaut) |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_sonicwall_`.

| Métrique | Labels | Signification |
|---|---|---|
| `version_info` | `version`, `model` | Valeur 1, quand la page de version répond. |
| `interface_link_up` | `interface`, `zone` | Lien, lu dans la chaîne d'état (`1 Gbps Full Duplex` = actif, `No link` = coupé). |
| `interface_down` | `interface`, `zone` | 1 quand l'interface est rattachée à une zone et n'a pas de lien. Les ports sans zone ou non attribués sont laissés tranquilles. |
| `interfaces`, `interfaces_down` | | Compteurs. |

La [règle intégrée](../alerting/rules.md#sonicwall-sonicos) qui s'applique :

- **Interface SonicWall sans lien** : pendant cinq minutes (Avis).

## La page de supervision

Le panneau Pare-feu indique quelles interfaces n'ont pas de lien et affiche la
version de SonicOS.

## Créer un utilisateur en lecture seule pour l'API SonicOS

1. Device → Settings → Administration → Audit/SonicOS API : vérifiez que HTTP Basic Access (RFC-2617) est activé, puis cliquez sur Accept.

2. Device → Users → Local Users & Groups → Add User : nommez-le comme suit, donnez-lui un long mot de passe aléatoire et ajoutez-le au groupe SonicWall Read-Only Admins.

    ```
    dumbmonit
    ```

3. Autorisez la gestion HTTPS sur l'interface par laquelle DumbMonit joint le pare-feu (Device → Network → Interfaces → Configure → Management).

4. Dans DumbMonit, saisissez l'adresse du pare-feu (sonicwall.lan) avec ce nom d'utilisateur et ce mot de passe.

### Permissions requises

- **Groupe SonicWall Read-Only Admins** — Le compte lit et ne modifie rien

!!! warning
    L'interface de gestion est livrée avec un certificat auto-signé : cochez « Accepter un certificat non vérifiable » sauf si vous avez installé le vôtre.

Documentation du fournisseur : https://www.sonicwall.com/support/knowledge-base/introduction-to-sonicos-api/kA1VN0000000FGr0AM

### Notes

- L'article de SonicWall indique que l'API est réservée aux comptes à
  privilèges complets. Si le compte en lecture seule est refusé (403) alors
  que le mot de passe est bon, SonicOS exige un privilège supérieur sur votre
  version : pesez-le avant de l'accorder.
- HTTP Basic doit être autorisé sous Audit/SonicOS API. Les noms de menu
  ci-dessus suivent SonicOS 7 ; ils varient légèrement entre 7.0 et 7.1.
- SonicOS 6.5 répond 406 sur le chemin d'état : cette intégration vise
  SonicOS 7.
- L'état des tunnels, de la HA et des licences n'est pas lu ; utilisez SNMP.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur et mot de passe | Le compte créé ci-dessus, envoyé en HTTP Basic. |

Adresse : le nom d'hôte ou l'IP du pare-feu, `hôte:port`, ou une URL. Options :

- **Port** (443) : le port HTTPS de l'interface de gestion.
- **Accepter un certificat non vérifiable** : le certificat propre du
  pare-feu est auto-signé.
- **Délai par requête** (15 s).

## Dépannage

« SonicWall refused the account (401) » : mauvais nom d'utilisateur ou mot de
passe, ou HTTP Basic n'est pas autorisé sous Audit/SonicOS API.

« SonicWall refused … (403) » : l'API SonicOS est désactivée, ou le compte ne
peut pas l'utiliser.

Aucun chiffre d'interface et une erreur « no /api/sonicos/… » : le firmware
n'offre pas le chemin d'état (SonicOS 6.5).

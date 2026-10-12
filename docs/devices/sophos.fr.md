# Sophos Firewall

Un Sophos Firewall (SFOS) via son API XML : le lien de chaque interface liée à
une zone, et quelles connexions IPsec sont activées.

C'est volontairement tout. L'API XML est une API de configuration : à part
l'état de connectivité en lecture seule de chaque interface, elle n'expose
aucun état en direct. `<Get><IPSecConnection>` renvoie si un administrateur a
activé une connexion, pas si le tunnel est monté ; la haute disponibilité, les
licences et abonnements et la version du firmware n'y sont pas du tout.
DumbMonit publie l'activation comme telle et ne la présente jamais comme un
état de tunnel.

Chaque minute, DumbMonit envoie deux requêtes à
`https://<firewall>:4444/webconsole/APIController` (champ de formulaire
`reqxml`, jamais dans l'URL, où le mot de passe finirait dans les logs) :
`<Get><Interface>` et `<Get><IPSecConnection>`. Chacune porte le nom
d'utilisateur et le mot de passe d'un compte dont le profil d'accès à
l'appareil est en lecture seule partout : le pare-feu impose ce profil à
l'API aussi.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la documentation de l'API
    XML de Sophos (les entités `Interface` et `IPSecConnection`, les messages
    de connexion et d'état), pas contre un pare-feu en fonctionnement. Le
    libellé exact de l'état d'interface (`Connected, 1000 Mbps…` ou
    `Disconnected`) vient de cette documentation et de retours de la
    communauté. Dites-nous ce qui casse.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_sophos_`.

| Métrique | Labels | Signification |
|---|---|---|
| `api_version_info` | `api_version` | Valeur 1 : la version d'API que le pare-feu annonce (`2000.2`), qui suit la release SFOS. |
| `interface_link_up` | `interface`, `zone` | 1 quand l'état indique Connected, 0 quand Disconnected ; absent quand l'état est autre chose. |
| `interface_enabled` | `interface`, `zone` | 0 quand l'interface est éteinte. |
| `interface_down` | `interface`, `zone` | 1 quand l'interface est allumée, liée à une zone, et n'a pas de lien. Les ports libres (zone None) sont laissés tranquilles. |
| `interfaces`, `interfaces_down` | | Compteurs. |
| `ipsec_connection_activated` | `connection` | 1 quand la connexion est activée dans la configuration. Pas l'état du tunnel. |
| `ipsec_connections` | | Compteur. |

La [règle intégrée](../alerting/rules.md#sophos-firewall) qui s'applique :

- **Interface Sophos sans lien** : une interface allumée et liée à une zone
  n'a eu aucun lien depuis cinq minutes (Avis).

Pour les compteurs de trafic, ajoutez aussi le pare-feu comme
[appareil SNMP](snmp.md) : SFOS répond à la MIB standard des interfaces.

## La page de l'appareil

Le panneau Firewall dit si chaque interface d'une zone a son lien, puis liste
ces interfaces et les connexions IPsec avec leur activation configurée,
étiquetée comme telle.

## Autoriser DumbMonit à lire l'API XML du Sophos Firewall

1. Sous Profiles > Device access, ajoutez un profil nommé DumbMonit read-only et réglez chaque permission sur Read-only.

2. Sous Authentication > Users, ajoutez un utilisateur nommé dumbmonit, de type Administrator, avec ce profil et un mot de passe long et aléatoire.

3. Sous Backup & firmware > API (Administration > API access depuis SFOS 22), activez la configuration de l'API et ajoutez l'adresse du serveur DumbMonit aux adresses IP autorisées.

4. Dans DumbMonit, saisissez l'adresse du pare-feu, par exemple « sophos.lan », avec le nom d'utilisateur et le mot de passe de dumbmonit. L'API écoute sur le port d'administration, 4444 par défaut, avec un certificat autosigné : cochez Accepter un certificat non vérifiable à moins d'avoir installé le vôtre.

!!! warning
    L'API XML est une API de configuration : elle rapporte le lien de chaque interface et si chaque connexion IPsec est activée, pas si le tunnel est monté. La haute disponibilité, les licences et la version du firmware n'y sont pas.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur et mot de passe | L'utilisateur `dumbmonit` avec le profil d'accès à l'appareil en lecture seule. |

Adresse : le nom d'hôte ou l'IP du pare-feu, `host:port`, ou une URL. Options :

- **Port** (4444) : le port d'administration.
- **Accepter un certificat non vérifiable** : le propre certificat du
  pare-feu est autosigné.
- **Délai par requête** (15 s).

## Dépannage

« Sophos Firewall refused the request (534 …) » : l'adresse DumbMonit n'est
pas parmi les adresses IP autorisées de la configuration de l'API.

« Sophos Firewall refused the login » : mauvais nom d'utilisateur ou mot de
passe, ou un mot de passe avec des caractères que le pare-feu rejette dans
les appels d'API ; un mot de passe long de lettres et de chiffres évite la
question.

« Sophos Firewall did not answer in XML » : mauvais port, ou l'API est
désactivée.

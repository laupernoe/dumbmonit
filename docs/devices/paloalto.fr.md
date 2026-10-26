# Palo Alto PAN-OS

Un pare-feu Palo Alto Networks (physique ou VM-Series) via l'API XML de
PAN-OS : interfaces rattachées à une zone qui ont perdu leur lien, paire HA
qui a perdu son pair ou désynchronisée, table des sessions qui se remplit,
CPU et mémoire du plan de gestion, licences et abonnements arrivant à
expiration, et validation de configuration en échec.

!!! warning "Bêta"

    Cette intégration a été construite à partir de la documentation de l'API XML
    de Palo Alto Networks, pas contre un pare-feu en fonctionnement. Chaque
    appel est une lecture, mais la forme de certaines réponses (état HA,
    `show system resources`, `show interface all`) est la moins documentée :
    dites-nous ce qui casse.

DumbMonit utilise un compte à rôle dont le rôle n'autorise que les requêtes
opérationnelles de l'API XML. La clé d'API est envoyée dans l'en-tête
`X-PAN-KEY`, jamais dans l'URL (l'ancienne forme `?key=` la laisserait dans
les journaux d'accès). Les requêtes partent en formulaires POST. Avec un nom
d'utilisateur et un mot de passe, DumbMonit demande une clé au pare-feu
(`type=keygen`, identifiants dans le corps de la requête), la garde une heure
et la redemande quand elle est refusée ; avec une clé générée par vos soins,
elle est utilisée telle quelle.

Chaque minute, DumbMonit exécute une commande opérationnelle requise, `show
system info`, et six optionnelles qu'un rôle plus restreint, ou un pare-feu
sans cluster, peut refuser sans faire échouer le reste : `show system
resources`, `show session info`, `show high-availability state`, `request
license info`, `show interface all` et `show jobs all`.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | Interface de gestion (HTTPS, par défaut) |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_paloalto_`.

| Métrique | Labels | Signification |
|---|---|---|
| `version_info` | `version`, `model`, `hostname` | Valeur 1. |
| `uptime_seconds` | | Temps écoulé depuis le démarrage. |
| `cpu_usage_percent`, `memory_used_percent` | | Plan de gestion, lus dans la sortie texte de `show system resources` (CPU : 100 moins l'inactivité ; mémoire : hors tampons et cache). Absents quand la sortie est illisible. |
| `sessions`, `sessions_max`, `session_usage_percent` | | Sessions actives, capacité de la table des sessions et part utilisée. |
| `session_setup_rate`, `packets_per_second`, `throughput_kbps` | | Débits issus de `show session info`. |
| `ha_enabled` | | 1 quand la HA est configurée. |
| `ha_state_info` | `state`, `peer_state` | Valeur 1, avec les états HA local et du pair. |
| `ha_local_functional` | | 0 quand l'état local est `suspended` ou `non-functional`. |
| `ha_peer_up` | | 1 quand le pair est actif ou passif et que le lien HA vers lui est établi. |
| `ha_in_sync` | | 1 quand la configuration courante est synchronisée. |
| `interface_link_up` | `interface`, `zone` | Lien physique des interfaces `ethernet` et `ae`. |
| `interface_down` | `interface`, `zone` | 1 quand l'interface est rattachée à une zone et n'a pas de lien. Les ports sans zone sont laissés tranquilles. |
| `interface_speed_mbps` | `interface`, `zone` | Tant que le lien est actif. |
| `interface_address_info` | `interface`, `description`, `address`, `vlan` | Valeur 1, le réseau IPv4 de l'interface (utilisé par la carte de topologie). |
| `interfaces_down` | | Compteur. |
| `license_status_info` | `license`, `status` | Valeur 1, pour chaque licence listée. |
| `license_expired` | `license` | 1 quand elle a expiré. |
| `license_expiry_seconds` | `license` | Jusqu'à l'expiration ; négatif une fois expirée. Les licences perpétuelles n'en ont pas. |
| `last_commit_ok` | | 1 quand la dernière validation terminée a réussi (une validation avec avertissements compte comme réussie). |

Les [règles intégrées](../alerting/rules.md#palo-alto-pan-os) qui s'appliquent :

- **Interface Palo Alto sans lien** : pendant cinq minutes (Avis).
- **Pair HA Palo Alto perdu** : pendant deux minutes (Avertissement).
- **HA Palo Alto désynchronisé** : pendant quinze minutes (Avis).
- **Membre HA Palo Alto non fonctionnel** : pendant cinq minutes (Avertissement).
- **Table des sessions Palo Alto qui se remplit** : au-dessus de 90 % pendant
  dix minutes (Avis).
- **CPU** et **mémoire** de gestion Palo Alto élevés : au-dessus de 90 %
  pendant quinze minutes (Avis).
- **Licence Palo Alto arrivant à expiration** : moins de 30 jours, ou expirée
  (Avis, rappelé chaque jour).
- **Validation Palo Alto en échec** : la dernière validation a échoué (Info,
  rappelée chaque jour).

## La page de supervision

Le panneau Pare-feu dit en une phrase ce qui ne va pas, affiche le CPU, la
mémoire, les sessions et l'état HA, puis liste les interfaces sans lien et les
licences qui expirent sous 60 jours ou ont expiré.

## Créer un compte API XML en lecture seule pour DumbMonit

1. Device → Admin Roles → Add : nommez le rôle comme suit. Dans l'onglet XML API, activez uniquement Operational Requests ; laissez désactivés tous les autres privilèges, ainsi que les onglets Web UI et Command Line.

    ```
    dumbmonit-ro
    ```

2. Device → Administrators → Add : nommez le compte comme suit, réglez Authentication sur un mot de passe, le type sur Role Based et le rôle sur celui ci-dessus.

    ```
    dumbmonit
    ```

3. Cliquez sur Commit : le rôle et le compte n'existent qu'une fois validés.

4. Dans DumbMonit, saisissez l'adresse du pare-feu (pa.lan) avec ce nom d'utilisateur et ce mot de passe, ou avec une clé d'API générée pour ce compte.

### Permissions requises

- **XML API → Operational Requests : activé** — show system info, resources, sessions, interfaces, état HA, jobs et licence info
- **Tout autre privilège XML API : désactivé** — Configuration, Commit, Export, Import, Report, Log et User-ID Agent ne sont pas nécessaires
- **Web UI et Command Line : désactivés** — Le compte ne se connecte jamais de façon interactive

!!! warning
    L'interface de gestion est livrée avec un certificat auto-signé : cochez « Accepter un certificat non vérifiable » sauf si vous avez installé le vôtre.

Documentation du fournisseur : https://docs.paloaltonetworks.com/pan-os/11-1/pan-os-panorama-api/get-started-with-the-pan-os-xml-api

### Notes

- Pour générer vous-même la clé au lieu de laisser DumbMonit le faire :

    ```
    curl -k -X POST "https://pa.lan/api/?type=keygen" -d "user=dumbmonit" --data-urlencode "password=..."
    ```

  Collez le `<key>` de la réponse comme clé d'API. Une clé générée pour un
  compte à rôle porte les limites de ce rôle.
- L'interface de gestion doit autoriser HTTPS, ce qui est le cas par défaut.
  Derrière une interface du plan de données, autorisez HTTPS dans son profil
  de gestion.
- Seul le plan de gestion est mesuré pour le CPU et la mémoire. La charge du
  plan de données n'est pas lue.
- Les chiffres GlobalProtect, des tunnels IPsec et du filtrage d'URL ne sont
  pas lus : les commandes ci-dessus ne les couvrent pas.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur et mot de passe | Le compte créé ci-dessus ; DumbMonit lui demande une clé d'API au pare-feu. |
| Clé d'API | Une clé déjà générée pour ce compte, envoyée en `X-PAN-KEY`. |

Adresse : le nom d'hôte ou l'IP du pare-feu, `hôte:port`, ou une URL. Options :

- **Port** (443) : le port HTTPS de l'interface de gestion.
- **Accepter un certificat non vérifiable** : le certificat propre du
  pare-feu est auto-signé.
- **Délai par requête** (15 s).

## Dépannage

« PAN-OS refused the credentials (403) » : clé erronée ou révoquée, mauvais
mot de passe, ou le rôle n'autorise pas les Operational Requests. Validez
(Commit) le rôle et le compte, puis réessayez.

« PAN-OS refused the request (16: Unauthorized request) » : le rôle n'a pas le
privilège Operational Requests.

Aucun chiffre HA, de licence, de session ou d'interface : le rôle ne peut pas
exécuter cette commande, ou le pare-feu n'a pas de cluster ; le reste est
quand même lu.

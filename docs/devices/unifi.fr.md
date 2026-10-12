# UniFi Network

Le réseau UniFi et ce qu'il gère : passerelles, commutateurs et points
d'accès hors ligne, isolés ou en attente d'adoption, mises à jour de
firmware en attente, les liens WAN et la connexion Internet telle que la
passerelle les voit, le nombre de clients, et les alarmes qu'UniFi a levées.
Ça fonctionne avec l'application UniFi Network d'une console UniFi OS
(Dream Machine, Cloud Gateway, Cloud Key) et avec un UniFi Network Server
auto-hébergé.

DumbMonit lit UniFi avec une clé d'API ou avec un compte local portant le
rôle **View Only**. Il ne redémarre, n'adopte, ne met à jour ni ne
provisionne jamais un appareil.

La panne à surveiller : **un point d'accès ou un commutateur qui se
déconnecte discrètement**. Les clients basculent vers le point d'accès
suivant, le réseau continue de fonctionner, et le trou de couverture ou le
lien montant perdu n'est remarqué que des jours plus tard. Pareil pour un
second lien WAN qui est tombé : rien ne casse jusqu'à ce que le premier
tombe aussi.

## Deux façons d'entrer

| Identifiant | API utilisée | Ce qu'elle lit |
|---|---|---|
| Clé d'API (console UniFi OS) | L'Integration API officielle, sur `/proxy/network/integration/v1` sur une console et `/integration/v1` sur un serveur auto-hébergé | Version, appareils et leur état, mises à jour de firmware, charge, mémoire et uptime de chaque appareil en ligne, nombre de clients |
| Compte View Only | L'API classique (`/api/s/<site>/stat/*`), sur `/proxy/network/api/…` sur une console | Tout ce qui précède, plus les clients par appareil et par type, les liens WAN, le statut WAN et Internet, et les alarmes |

Le statut WAN, Internet et des alarmes n'existe que dans l'API classique.
Avec une clé, DumbMonit demande aussi à l'API classique le statut WAN et
Internet avec la même clé ; si le contrôleur la refuse, ces séries sont
simplement absentes, sans erreur.

Les alarmes sont les entrées du journal système « critical » d'UniFi sur
les dernières 24 heures (UniFi Network 8 et plus récent), ou les alarmes
non archivées sur les versions plus anciennes. C'est le seul `POST` que
DumbMonit envoie, et il ne fait que lire.

Une session ouverte avec le compte View Only est gardée d'une sonde à
l'autre et rouverte quand elle expire : UniFi n'enregistre pas une nouvelle
connexion chaque minute.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_unifi_`. Les séries
d'appareil portent `device` (son nom, ou son adresse MAC quand il n'en a
pas), `mac`, `model` et `type` (`gateway`, `switch`, `access_point`,
`other`).

| Métrique | Quoi | Labels |
|---|---|---|
| `info` | valeur 1 | `version`, `api` (`integration` ou `classic`) |
| `devices` | appareils par état : `online`, `offline`, `pending`, `updating`, `adopting`, `adoption_failed`, `isolated`, `other` | `state` |
| `devices_upgradable` | appareils adoptés avec une mise à jour de firmware disponible | |
| `device_state` | 0 en ligne, 1 hors ligne, 2 en attente d'adoption, 3 en mise à jour ou provisionnement, 4 en cours d'adoption, 5 adoption échouée, 6 isolé, 7 autre | labels d'appareil |
| `device_up` | 1 en ligne ou en mise à jour, 0 hors ligne, isolé ou adoption échouée ; absent pour un appareil pas encore adopté | labels d'appareil |
| `device_upgradable` | 1 quand une mise à jour de firmware est disponible | labels d'appareil, `firmware` |
| `device_uptime_seconds`, `device_cpu_percent`, `device_memory_percent` | appareils en ligne seulement | labels d'appareil |
| `device_clients` | clients connectés à l'appareil (compte View Only) | labels d'appareil |
| `wan_link_up` | 1 quand un port WAN de la passerelle est actif ; un port désactivé n'a aucune série | labels d'appareil, `wan` (`wan1`, `wan2`) |
| `wan_up`, `internet_up` | le statut WAN et Internet de la passerelle ; absent sans passerelle UniFi | |
| `internet_latency_seconds` | latence mesurée par la passerelle | |
| `wan_availability_percent` | disponibilité de chaque WAN sur les dernières 24 heures | `wan` |
| `subsystem_status` | 0 ok, 1 avertissement, 2 erreur, 3 inconnu, pour `wlan`, `wan`, `www`, `lan`, `vpn` | `subsystem` |
| `clients` | clients sur le site | |
| `clients_by_type` | `wireless`, `wired`, `guest`, `vpn` (compte View Only) | `type` |
| `alarms` | entrées du journal système critique des dernières 24 heures, ou alarmes non archivées | |
| `scrape_errors` | appels qui ont échoué à la dernière sonde | |

Les [règles intégrées](../alerting/rules.md#unifi-network) qui
s'appliquent : appareil UniFi hors ligne, Internet UniFi en panne, lien WAN
UniFi en panne, CPU de passerelle UniFi élevé, appareil UniFi en attente
d'adoption, mise à jour de firmware UniFi disponible, alarmes UniFi, plus
Appareil inaccessible.

## Appareils

`/rest/user` (l'API classique, lue avec l'un ou l'autre identifiant) liste
chaque client que le contrôleur a jamais vu — la plupart d'un réseau
domestique, en pratique : téléphones qui passent, Wi-Fi invité, appareils
connectés qui ne reviennent jamais. Suivre tous ces appareils pour la
péremption enterrait l'alerte sous des appareils qui ne reviennent
simplement jamais, pas des appareils devenus silencieux. Par défaut, seuls
les clients que l'utilisateur a marqués d'une façon ou d'une autre
comptent : nommés dans UniFi, réservés sur une IP fixe, ou notés. L'option
**Clients à surveiller en plus des marqués** ajoute des adresses MAC ou
noms de client spécifiques, ou `all` pour suivre chaque client (non
recommandé sur autre chose qu'un petit réseau largement statique).

Les clients suivis sont publiés sous la famille générique partagée par
chaque intégration DumbMonit qui suit les appareils clients —
`dumbmonit_client_device_*`, préfixée sans `unifi`, documentée en entier
sous [Appareils clients](../alerting/rules.md#client-devices) :

| Métrique | Quoi | Labels |
|---|---|---|
| `last_seen_timestamp_seconds` | Dernière connexion, comme `/rest/user` la rapporte | `device`, `type` (`Wired`, `Wireless` ou `Unknown`), `os` (vide), `user` (vide), `kind` |
| `stale_seconds` | Positif une fois qu'un client suivi a dépassé `device_stale_days` (3 par défaut) sans se connecter ; alimente la règle [Appareil client périmé](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (toujours `connection` : UniFi n'a pas de sauvegarde à rapporter) |

## La page de l'appareil

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais UniFi. Il dit d'abord ce qui ne va pas (appareils
hors ligne, Internet ou un lien WAN en panne, alarmes), puis les appareils
par type avec leur état, firmware et charge, et les clients, puis le
tableau des clients suivis avec leur dernière connexion.

## Comment cela a été validé

La connexion, le rôle View Only, les réponses d'erreur (mot de passe
refusé, site inconnu, clé refusée, session expirée) et les réponses d'un
contrôleur sans appareils ont été vérifiées contre UniFi Network 10.6.106,
auto-hébergé. Aucun appareil n'a pu y être adopté : la liste d'appareils, la
santé d'un site équipé, les clients connus (`/rest/user`) et les charges
utiles de l'Integration API sont validées contre la documentation
officielle.

## Créer un accès en lecture seule dans UniFi Network

1. Sur une console UniFi OS (Dream Machine, Cloud Gateway, Cloud Key), vous pouvez créer une clé d'API : dans UniFi Network, Settings → Control Plane → Integrations → Create API Key. Nommez-la d'après DumbMonit et copiez-la maintenant : elle n'est montrée qu'une fois. La clé lit les appareils, leur charge et le nombre de clients via l'Integration API officielle.

2. Pour le statut WAN, Internet et des alarmes, et sur un UniFi Network Server auto-hébergé, utilisez plutôt un compte local avec le rôle View Only : Settings → Admins & Users → Create New. Nommez-le comme suit, donnez-lui un mot de passe long et aléatoire, restreignez-le à l'accès local, et réglez son rôle UniFi Network sur View Only. View Only lit tout et ne peut rien changer.

    ```
    dumbmonit
    ```

3. Dans DumbMonit, saisissez l'adresse de la console ou du serveur, par exemple « unifi.lan » (port 443 sur une console) ou « unifi.lan:8443 » pour un serveur auto-hébergé, et collez la clé ou le compte. Si le réseau n'est pas le site par défaut, réglez l'option Site sur le nom court affiché dans la barre d'adresse après /manage/ ou /network/.

4. DumbMonit ne fait que lire. Il ne redémarre, n'adopte, ne met à jour ni ne provisionne jamais un appareil, et n'acquitte jamais une alarme.

!!! warning
    N'utilisez pas le compte propriétaire ou un compte cloud UI.com : un mot de passe qui fuit ouvrirait chaque console liée à lui. Laissez la double authentification désactivée pour ce seul compte local View Only, car un serveur de surveillance ne peut pas taper un code. Les consoles utilisent un certificat autosigné par défaut : activez « Accepter un certificat non vérifiable » sauf si vous avez installé le vôtre. UniFi connaît chaque client qui s'est déjà connecté, la plupart des téléphones qui passent : seuls ceux nommés, réservés sur une IP fixe, ou notés dans UniFi sont surveillés pour la péremption par défaut, pour éviter une alerte pour chaque téléphone de visiteur. Listez-en plus dans l'option Clients à surveiller, ou saisissez all.

## Identifiants

| Identifiant | Champs |
|---|---|
| Clé d'API (console UniFi OS) | Une clé créée dans Settings → Control Plane → Integrations, envoyée comme `X-API-KEY`. |
| Compte View Only | Le nom d'utilisateur et le mot de passe du compte local View Only. |

Adresse : un nom d'hôte ou une IP (`unifi.lan`), `host:port`
(`unifi.lan:8443` pour un serveur auto-hébergé), ou une URL complète ; une
adresse copiée depuis le navigateur, avec `/manage/…` ou `/network/…`, est
acceptée. Options : site (`default`), port (443), vérification du
certificat, délai de requête (15 s), clients à surveiller en plus des
marqués (vide, ou `all`), et péremption d'appareil (`device_stale_days`,
3 jours par défaut).

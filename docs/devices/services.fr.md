# Services : HTTP, TCP, DNS, ping, TLS, NTP, UDP, SSH, SMTP, IMAP, POP3, SQL, MQTT, WebSocket

Quinze types de surveillance couvrent des **services** plutôt que du matériel, à la
façon d'Uptime Kuma : une page web, un port, un nom DNS, un hôte, un
certificat, un serveur de temps, un relais mail ou une boîte aux lettres, une base de données, un
courtier MQTT, un point de terminaison WebSocket, un port UDP, un serveur SSH. Chaque service obtient
une barre d'historique, un temps de réponse et un pourcentage de
disponibilité sur sa page de surveillance.

Les cinq derniers vont plus loin que l'ouverture d'une connexion : ils
démarrent une vraie session dans le propre protocole du service. C'est la
différence entre « le port 5432 est ouvert » et « la base de données
accepte toujours mon compte et répond à une requête ». NTP se trouve
entre les deux : une seule requête et réponse, dans le propre protocole
de NTP, assez pour lire l'horloge du serveur sans session.

## Venir d'Uptime Kuma

**Surveillances > ⋯ > Importer depuis Uptime Kuma**, aussi sur
**Surveillances > Ajouter une surveillance > Import** (administrateurs), lit la
base d'Uptime Kuma 1.x ou 2.x, ou la sauvegarde JSON de la 1.x. Rien n'est créé
avant que vous ayez vu l'aperçu : ce qui sera créé, et ce qui est ignoré avec la
raison. Cochez les surveillances voulues, puis importez-les en un clic.

Uptime Kuma 2.x n'a plus d'export de sauvegarde : exportez donc sa base. Arrêtez
le conteneur (pour que le fichier soit complet), copiez `kuma.db` hors du dossier
de données, puis choisissez-le dans la fenêtre.

```bash
docker stop uptime-kuma
docker cp uptime-kuma:/app/data/kuma.db .
docker start uptime-kuma
```

- Le fichier est envoyé, lu une seule fois en lecture seule depuis une copie
  temporaire qui n'est jamais rattachée à la base de DumbMonit, puis supprimé
  aussitôt. Il peut peser jusqu'à 512 Mo (il contient tout l'historique des
  mesures) ; seuls les moniteurs, groupes, étiquettes, intervalles et le nombre de
  notifications en sont lus.
- Uptime Kuma 2 installé sur **MariaDB** n'est pas pris en charge : seul le
  fichier SQLite `kuma.db` peut être importé.
- La sauvegarde JSON de la 1.x (**Réglages > Sauvegarde > Exporter**, 4 Mo au
  plus) fonctionne toujours, avec le même aperçu.

| Uptime Kuma | DumbMonit |
|---|---|
| HTTP(s), HTTP(s) - Mot-clé, HTTP(s) - Requête JSON | [HTTP](#http) : codes acceptés, méthode, mot-clé (et son inverse), chemin JSON et valeur attendue, redirections, délai, erreurs TLS ignorées |
| Ping | [Ping](#ping) |
| Port TCP | [TCP](#tcp), sous la forme `hôte:port` |
| DNS | [DNS](#dns) : type d'enregistrement et résolveur |
| Push | [Battement de cœur](push.md), période attendue = l'intervalle de la surveillance. Il reçoit une **nouvelle** URL : mettez à jour la tâche qui l'appelle |
| Groupe | Un dossier (les groupes imbriqués donnent `Groupe/Sous-groupe`) |
| Étiquettes | Étiquettes de la surveillance : `nom=valeur`, ou `nom=true` pour une étiquette sans valeur |
| En pause | Créé désactivé |

Le nom et l'intervalle sont conservés (10 s au minimum). Tout le reste est
ignoré et listé : Docker et les autres types sans équivalent, et les surveillances
en mode inversé. Une surveillance déjà suivie (même type, même adresse) est ignorée
aussi : on peut donc importer deux fois le même fichier sans risque.

**Non importés :** les notifications (configurez vos canaux dans
[Notifications](../notifications.md) ; l'aperçu indique combien le fichier en
contenait), ainsi que les identifiants, en-têtes personnalisés et corps de
requête, qui ne sont jamais lus par DumbMonit. Si un site surveillé demande une
connexion ou un en-tête, ajoutez-le ensuite sur la surveillance.

## Comment fonctionne l'état d'un service

Un service peut répondre et être en panne quand même : un `500`, un mot-
clé manquant, un certificat expiré. Ces surveillances écrivent donc un point
à chaque vérification, `probe_success` étant 1 ou 0, plutôt que de
devenir silencieux comme un appareil matériel. C'est ce qui rend un
pourcentage de disponibilité possible :
`avg_over_time(dumbmonit_probe_success[30d])`.

| Situation | Ce qui se passe |
|---|---|
| Option ou adresse invalide, sockets ICMP non autorisées (voir Ping) | Erreur de configuration : rien n'est écrit, la page de la surveillance montre l'erreur, aucune alerte « en panne ». |
| Connexion refusée, délai dépassé, TLS refusé, `500`, mot-clé absent, perte totale de paquets, mot de passe refusé, requête échouée | `probe_success = 0` : la panne est enregistrée, datée et comptée. |
| Tout va bien | `probe_success = 1`. |

Chaque surveillance a son propre délai (`timeout_seconds`, 5 s par défaut,
60 s au plus), plus court que le `DUMBMONIT_PROBE_TIMEOUT_SECS` du
serveur : interrompu par le planificateur, il ne pourrait pas écrire son
zéro.

Règles intégrées qui s'appliquent aux quinze : Service en panne (3
minutes), Service instable (plus de six changements d'état en trente
minutes), Service lent (plus de 3 s pendant dix minutes). Pour chaque
vérification qui lit un certificat — `http`, `tls`, `smtp`, `imap`, `pop3`, `mqtt`,
`websocket` : Certificat arrivant à expiration (14 jours) et Certificat
expiré.

Un mot de passe refusé n'est jamais une panne du service : il est
enregistré comme `reason="auth"`, l'alerte dit donc quoi corriger.

### Métriques

Toutes des jauges, toutes préfixées `dumbmonit_`, toutes étiquetées
`probe="http|tcp|dns|ping|tls|smtp|postgres|mysql|mqtt|websocket|ntp|udp|ssh|imap|pop3|snmp_oid|radius|grpc"` en
plus de `target`, `host` et `tag_*`.

| Métrique | Surveillances | Signification |
|---|---|---|
| `probe_success` | tous | 1 si le service répond correctement, 0 sinon. |
| `probe_duration_seconds` | tous | Durée totale de la vérification. |
| `probe_failure_info` | tous | Présence (1) avec un label `reason` : `dns`, `connect`, `timeout`, `tls`, `cert_expired`, `status`, `keyword`, `json`, `body`, `packet_loss`, `record`, `auth`, `protocol`, `query`, `payload`, `unsynchronized`, `clock_offset`, `host_key`, `threshold`, `not_found`, `bad_secret`, `unhealthy`. |
| `probe_http_status_code` | http, websocket | Code de statut obtenu. |
| `probe_http_first_byte_seconds` | http | Temps jusqu'aux en-têtes de réponse. |
| `probe_http_phase_seconds{phase}` | http | Temps de la requête mesurée réparti en `dns`, `connect`, `tls`, `processing` et `transfer` ; les cinq somment toujours au temps de la requête (voir [Décomposition du temps de réponse](#decomposition-du-temps-de-reponse)). |
| `probe_http_content_bytes` | http | Taille du corps. |
| `probe_http_security_header{header}` | http | 0/1 par étiquette `header` (https seulement) : HSTS, nosniff, CSP, X-Frame-Options, frame-ancestors. |
| `probe_connect_seconds` | tous sauf dns, ping, ntp et udp | Établissement de connexion (connexion incluse pour les bases de données). |
| `probe_tls_handshake_seconds` | tls, http, smtp, imap, pop3, mqtt, websocket | Négociation TLS seule. |
| `probe_tls_version_info` | tls, http, smtp, imap, pop3, mqtt, websocket | Présence ; label `version`. |
| `probe_ssl_cert_expiry_days` | tls, http, smtp, imap, pop3, mqtt, websocket | Jours avant expiration, négatif si expiré. |
| `probe_ssl_cert_valid` | tls, http, smtp, imap, pop3, mqtt, websocket | 1 si la chaîne mène à une autorité connue. |
| `probe_ssl_cert_issuer_info` | tls, http, smtp, imap, pop3, mqtt, websocket | Présence ; label `issuer`. |
| `probe_dns_lookup_seconds` | dns | Temps de résolution. |
| `probe_dns_answer_records` | dns | Enregistrements du type demandé. |
| `probe_dns_answer_fingerprint` | dns | Avec « Alerter quand la réponse change » : une empreinte 32 bits de la réponse triée. Sa valeur ne signifie rien ; seuls ses changements comptent. |
| `probe_icmp_rtt_seconds`, `_min_seconds`, `_max_seconds` | ping | Aller-retour moyen, le plus court et le plus long. |
| `probe_icmp_packet_loss_ratio` | ping | Perte entre 0 et 1. |
| `probe_icmp_packets_sent`, `probe_icmp_packets_received` | ping | Échos envoyés et reçus. |
| `probe_smtp_greeting_seconds` | smtp | Temps jusqu'à la bannière `220`. |
| `probe_smtp_ehlo_seconds` | smtp | Aller-retour de la commande `EHLO`. |
| `probe_smtp_capabilities` | smtp | Extensions annoncées. |
| `probe_smtp_authenticated` | smtp | 1 quand `AUTH` a réussi. |
| `probe_sql_query_seconds` | postgres, mysql | Exécution de la requête seule. |
| `probe_sql_rows` | postgres, mysql | Lignes renvoyées. |
| `probe_sql_value` | postgres, mysql | Première colonne de la première ligne, quand c'est un nombre. |
| `probe_mqtt_connack_seconds` | mqtt | Temps jusqu'au `CONNACK`. |
| `probe_mqtt_suback_seconds` | mqtt | Temps jusqu'au `SUBACK`. |
| `probe_mqtt_message_bytes` | mqtt | Taille du message retenu. |
| `probe_mqtt_message_value` | mqtt | Son contenu, quand c'est un nombre. |
| `probe_ws_handshake_seconds` | websocket | Poignée de main d'ouverture. |
| `probe_udp_response_seconds`, `probe_udp_response_bytes` | udp | Temps de réponse et taille de la réponse (seulement quand une réponse arrive). |
| `probe_ssh_banner_seconds` | ssh | Temps jusqu'à la bannière `SSH-2.0-...`. |
| `probe_ssh_protocol_info{version}` | ssh | Présence ; version du protocole dans le label `version`. |
| `probe_ssh_software_info{software}` | ssh | Présence ; logiciel du serveur dans le label `software` (`OpenSSH_9.6p1`). |
| `probe_ssh_kex_seconds` | ssh | Durée du début de l'échange de clés (seulement quand la clé d'hôte est lue). |
| `probe_ssh_host_key_fingerprint` | ssh | Avec une clé d'hôte attendue ou « Alerter quand la clé d'hôte change » : une empreinte 32 bits de la clé d'hôte. Sa valeur ne signifie rien ; seuls ses changements comptent. |
| `probe_ssh_host_key_info{fingerprint}` | ssh | Présence ; l'empreinte `SHA256:...`, telle que l'affiche `ssh-keygen -lf`. |
| `probe_ssh_host_key_match` | ssh | 1 si la clé d'hôte est celle attendue, 0 sinon. Seulement avec une clé d'hôte attendue. |
| `probe_imap_greeting_seconds`, `probe_imap_login_seconds` | imap | Temps jusqu'à la bannière `* OK`, et aller-retour de `LOGIN`. |
| `probe_imap_authenticated` | imap | 1 quand `LOGIN` a réussi. |
| `probe_pop3_greeting_seconds`, `probe_pop3_login_seconds` | pop3 | Temps jusqu'à la bannière `+OK`, et durée de `USER` + `PASS`. |
| `probe_pop3_authenticated` | pop3 | 1 quand l'identification a réussi. |
| `probe_ws_message_bytes` | websocket | Taille de la trame reçue. |
| `probe_snmp_query_seconds` | snmp_oid | Aller-retour de la lecture. |
| `probe_snmp_value` | snmp_oid | Valeur de l'objet, si elle est numérique. |
| `probe_radius_response_seconds` | radius | Aller-retour de l'Access-Request. |
| `probe_radius_response_code` | radius | Code de la réponse : 2 accepté, 3 refusé, 11 défi. |
| `probe_grpc_check_seconds` | grpc | Durée de l'appel `Health/Check`. |
| `probe_grpc_serving_status` | grpc | `ServingStatus` renvoyé : 1 SERVING, 2 NOT_SERVING, 3 SERVICE_UNKNOWN. |
| `probe_grpc_status_code` | grpc | Code `grpc-status` de la réponse (0 : OK). |

## HTTP {#http}

**Site web ou API web (HTTP)** — vérifie qu'une page ou une API répond,
avec le bon code de statut, le bon contenu, et un certificat valide.
Exemples : un site web, une page de santé d'application, une API REST,
une interface de service auto-hébergé.

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | Page web ou API en HTTPS (port par défaut) |
| 80 | TCP | Seulement avec une adresse http:// (port par défaut) |

### Configuration

1. Dans l'adresse, collez l'URL complète de la page à surveiller.

2. Préférez une page légère comme « /health » ou « /status » plutôt que la page d'accueil.

3. Si la page demande une connexion, choisissez un identifiant : nom d'utilisateur et mot de passe (basique) ou jeton (Bearer).

4. Au besoin, exigez un mot-clé dans la page ou une valeur dans une réponse JSON, dans les options.

**Droits requis**

- **Accès en lecture à l'URL surveillée** : Seulement si la page demande une connexion

!!! warning
    N'écrivez jamais un mot de passe ou un jeton dans les options : utilisez le champ identifiant, qui est chiffré.

Sans « http:// » ni « https:// », HTTPS est supposé. Par défaut, tout code de statut entre 200 et 299 convient. En HTTPS, le certificat est lu automatiquement : vous serez averti avant son expiration. Un mot de passe ou un jeton écrit dans les options est copié en texte clair à chaque mesure.

Identifiants : aucun, nom d'utilisateur / mot de passe (authentification
basique) ou jeton d'API (`Authorization: Bearer`).

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `method` | Méthode HTTP | `GET` | GET convient à presque chaque cas. HEAD évite de télécharger la page quand seul le code de statut compte. Choix : GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS. |
| `accepted_status` | Codes de statut acceptés | `200-299` | Codes ou plages considérés normaux, séparés par des virgules. Tout autre code compte comme en panne. Exemple : `200-299,301,404`. |
| `keyword` | Mot-clé attendu | *(vide)* | Texte qui doit apparaître dans la page. S'il manque, le service est rapporté en panne même si la page répond. |
| `keyword_absent` | Le mot-clé doit être absent | `false` | Inverse la vérification : la présence du mot-clé signale un échec. Utile pour une page d'erreur qui répond 200. |
| `keyword_case_sensitive` | Respecter la casse du mot-clé | `false` | Par défaut, majuscules et minuscules sont traitées pareil. |
| `json_path` | Chemin JSON à vérifier | *(vide)* | Pour une réponse JSON : chemin de la valeur à vérifier. Remplissez-le avec la valeur attendue, jamais l'un sans l'autre. Exemple : `$.status`. |
| `json_expect` | Valeur JSON attendue | *(vide)* | Valeur que le chemin JSON ci-dessus doit avoir, par exemple « ok » ou « true ». |
| `headers` | En-têtes supplémentaires | *(vide)* | Un ou plusieurs en-têtes « Nom: valeur », séparés par « \| ». Ne mettez pas de secrets ici : les options sont visibles dans les graphiques. |
| `body` | Corps de requête | *(vide)* | Contenu envoyé avec la requête, pour les méthodes qui en attendent un (POST, PUT…). |
| `follow_redirects` | Suivre les redirections | `true` | Désactivez pour surveiller la redirection elle-même, par exemple un 301 vers HTTPS. |
| `max_redirects` | Redirections maximum suivies | `10` | Entre 1 et 20. Sans effet si les redirections ne sont pas suivies. |
| `insecure_tls` | Accepter un certificat non vérifiable | `false` | La vérification n'échoue plus sur un certificat autosigné ou émis par une autorité privée. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Par défaut la vérification refuse les adresses que seul l'hôte DumbMonit lui-même peut atteindre (127.0.0.1, ::1, 169.254.x.x). Activez ceci pour surveiller un service tournant sur l'hôte DumbMonit. Les adresses LAN privées (10.x, 192.168.x) sont toujours autorisées. |
| `check_certificate` | Lire le certificat | `true` | En HTTPS, enregistre aussi la date d'expiration du certificat pour que vous soyez averti avant qu'il expire. |
| `max_body_bytes` | Octets maximum lus | `524288` | Au-delà, le reste de la page n'est pas téléchargé. Le mot-clé et le chemin JSON ne sont recherchés que dans cette partie. |
| `user_agent` | Identité annoncée (User-Agent) | `DumbMonit/<version>` | Nom que la vérification donne au serveur. Changez-le si le serveur filtre les robots. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Décomposition du temps de réponse {#decomposition-du-temps-de-reponse}

Sur la page du moniteur, le panneau **Décomposition du temps** montre où chaque
contrôle passe son temps, en aires empilées sur la période choisie, avec une barre
proportionnelle et une légende (nom, durée, part) pour le dernier contrôle. Les
mêmes valeurs sont dans la métrique `probe_http_phase_seconds`, une série par
étiquette `phase` :

| Phase | Ce qu'elle couvre |
|---|---|
| `dns` | Résolution du nom d'hôte. Nulle pour une adresse IP. |
| `connect` | Ouverture de la connexion TCP. |
| `tls` | Négociation TLS (HTTPS seulement). |
| `processing` | Attente du serveur : de la requête envoyée aux en-têtes de réponse. |
| `transfer` | Lecture du corps de la réponse. Nulle sauf si un mot-clé ou un chemin JSON exige le corps. |

Les cinq séries existent toujours et somment au temps écoulé entre l'envoi de la
requête et la fin de la lecture du corps, pour la dernière requête du contrôle (le
dernier saut quand les redirections sont suivies). Chaque contrôle ouvre une
connexion neuve : `dns`, `connect` et `tls` sont donc mesurés à chaque fois, au
lieu de rester à zéro sur une connexion réutilisée.

Lire le graphique : une bande `dns` haute désigne le résolveur, `connect` le
chemin réseau ou un pare-feu, `tls` la chaîne de certificats ou le coût de la
négociation, `processing` l'application elle-même. La séparation TCP / TLS repose
sur la durée de négociation relevée par la lecture du certificat juste avant la
requête, plafonnée au temps de connexion observé ; si **Lire le certificat** est
désactivé, la négociation TLS reste comprise dans `connect`. Un contrôle en échec
ne publie pas de phases : le graphique montre alors un trou.

## TCP {#tcp}

**Port réseau (TCP)** — vérifie qu'un port accepte les connexions : SSH,
base de données, partage de fichiers, serveur de jeu, imprimante réseau.

### Ports réseau à ouvrir

Le port est celui que vous écrivez dans l'adresse ou dans l'option « Port » ; il n'y a pas de valeur par défaut. Du serveur DumbMonit (ou de son agent relais) vers l'équipement, ouvrez ce port TCP.

### Configuration

1. Dans l'adresse, écrivez l'hôte et le port séparés par deux-points : « nas.home.lan:22 », ou « [fd00::1]:445 » en IPv6.

2. Ou laissez le port hors de l'adresse et remplissez l'option « Port ».

!!! warning
    Un port ouvert ne prouve pas que l'application fonctionne : pour un service web, utilisez le type « Site web ou API web ».

Seule l'ouverture de la connexion est testée : aucune donnée n'est envoyée au service, il n'en est donc pas affecté.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `port` | Port | *(vide)* | Port à ouvrir, si l'adresse ne le donne pas déjà comme « hôte:port ». L'un des deux est requis. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Par défaut la vérification refuse les adresses que seul l'hôte DumbMonit lui-même peut atteindre (127.0.0.1, ::1, 169.254.x.x). Activez ceci pour surveiller un service tournant sur l'hôte DumbMonit. Les adresses LAN privées (10.x, 192.168.x) sont toujours autorisées. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

## DNS {#dns}

**Nom de domaine (DNS)** — vérifie qu'un nom se résout, et qu'il pointe
vers la bonne adresse. Exemples : votre nom de domaine, un nom interne
servi par votre Pi-hole ou AdGuard, un enregistrement MX.

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 53 | UDP | Requête DNS (port par défaut) |
| 53 | TCP | Requête DNS trop grosse pour UDP (port par défaut) |

### Configuration

1. Dans l'adresse, écrivez le nom à résoudre, sans « http:// » : « www.example.com ».

2. Choisissez un autre type d'enregistrement dans les options si vous ne voulez pas d'adresse IPv4 (enregistrement A).

3. Pour surveiller votre propre serveur DNS, saisissez son adresse IP dans « Serveur DNS à interroger ».

4. Pour détecter un détournement, listez dans « Valeurs attendues » les adresses que la réponse doit contenir.

!!! warning
    Indiquez le serveur DNS par son adresse IP, pas par un nom.

Par défaut, la vérification interroge le résolveur du système. Si le serveur interrogé cesse de répondre, la vérification échoue.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `record_type` | Type d'enregistrement | `A` | A pour une adresse IPv4, AAAA pour IPv6, MX pour le mail, CNAME pour un alias… Choix : A, AAAA, CNAME, MX, TXT, NS, SOA, SRV, PTR, CAA. |
| `resolver` | Serveur DNS à interroger | *(vide)* | Adresse IP d'un résolveur, port optionnel. Vide : le résolveur système. Pratique pour vérifier votre propre serveur DNS. Exemple : `1.1.1.1` ou `10.0.0.1:5353`. |
| `expect` | Valeurs attendues | *(vide)* | Valeurs qui doivent toutes apparaître dans la réponse, séparées par des virgules. Vide : seule la résolution est vérifiée. |
| `expect_mode` | Comparaison | `contains` | `contains` : chaque valeur attendue doit apparaître quelque part dans la réponse. `exact` : la réponse doit porter ces valeurs et rien d'autre. |
| `forbid` | Valeurs interdites | *(vide)* | Valeurs qui ne doivent jamais apparaître dans la réponse, séparées par des virgules. Vérifiées avant les attendues. |
| `alert_on_change` | Alerter quand la réponse change | `false` | Enregistre une empreinte de la réponse triée et alerte chaque fois qu'elle change, sans avoir à écrire les valeurs attendues. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Affirmer ce qu'une zone répond

`expect` seul attrape l'enregistrement qui a disparu. Il n'attrape pas
celui qui a été *ajouté* : une zone détournée garde souvent le `A`
légitime et en met un second à côté, et un client atteint alors l'un ou
l'autre au hasard.

- `expect_mode = exact` refuse toute valeur que l'attente ne couvre pas.
  L'ordre n'importe pas — DNS n'en garde pas.
- `forbid` nomme les valeurs qui ne doivent jamais revenir : l'adresse
  d'un ancien hôte, un `TXT` de validation expiré, un serveur mail
  démantelé. Il est vérifié en premier, il gagne donc sur `expect`.

Les deux comparent sans tenir compte de la casse, ignorent le point
final d'un nom et les guillemets autour d'un `TXT`, et traitent une
valeur attendue comme trouvée quand elle apparaît à l'intérieur d'un
enregistrement — un `MX` correspond donc sur `mail.example.com` sans que
vous ayez à copier sa priorité.

### Alerter quand la réponse change {#alerting-when-the-answer-changes}

Écrire les valeurs attendues n'est pas toujours pratique : un nom servi
par un CDN, des enregistrements que quelqu'un d'autre gère, ou
simplement une zone dont vous voulez entendre parler chaque fois qu'elle
bouge. Cochez **Alerter quand la réponse change** : chaque vérification
enregistre alors une empreinte de la réponse, et la règle intégrée
[Réponse DNS changée](../alerting/rules.md#reverse-proxies-and-domains)
se déclenche quand elle diffère de la précédente. L'alerte se résout
d'elle-même une heure plus tard ; la nouvelle réponse est la référence
à partir de là.

L'empreinte couvre les valeurs de la réponse, triées et sans doublons,
avec la chaîne CNAME incluse : un round-robin qui renvoie les mêmes
adresses dans un autre ordre ne la change pas, ni un TTL. Un nom dont la
réponse change à chaque requête (un grand pool comme `pool.ntp.org`, ou
un DNS géographique qui répond différemment d'un résolveur à l'autre)
alerterait tout le temps : laissez l'option désactivée pour ceux-là, ou
interrogez un résolveur fixe.

## Ping {#ping}

**Hôte accessible (ping)** — envoie quelques échos ICMP et mesure le
temps de réponse et la perte de paquets. Exemples : routeur ou passerelle
maison, point d'accès Wi-Fi, imprimante, machine sans agent ni SNMP, hôte
distant.

### Ports réseau à ouvrir

Le ping utilise l'écho ICMP, qui n'a pas de port : autorisez la requête et la réponse d'écho ICMP entre le serveur DumbMonit (ou son agent relais) et l'équipement.

### Configuration

1. Dans l'adresse, écrivez le nom d'hôte ou l'adresse IP : « 192.168.1.1 » ou « router.home.lan ».

2. Baissez « Perte tolérée » dans les options pour être averti avant que la perte soit totale.

!!! warning
    Dans Docker, gardez les lignes sysctls du docker-compose.yml fourni, sinon la vérification ne peut pas envoyer de ping.

À chaque interrogation, la vérification envoie quatre échos et mesure le temps de réponse et la part de paquets perdus. Par défaut, seule une perte totale compte comme panne ; la perte partielle reste visible dans les graphiques. Certains équipements ignorent les pings à dessein.

```yaml
services:
  dumbmonit:
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Hors Docker, `sysctl -w net.ipv4.ping_group_range="0 2147483647"` sur
l'hôte fait la même chose, et une socket brute (`NET_RAW`, root)
fonctionne aussi.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `count` | Nombre d'échos | `4` | Paquets envoyés à chaque sonde, de 1 à 20. |
| `packet_timeout_ms` | Attente par paquet (ms) | `1000` | Temps accordé à chaque réponse, de 50 à 10 000 ms. |
| `interval_ms` | Écart entre deux échos (ms) | `100` | Pause entre deux échos, de 0 à 5 000 ms. |
| `payload_bytes` | Taille de charge utile (octets) | `56` | Données portées par chaque écho, de 0 à 1 400 octets. |
| `ip_version` | Version IP | `auto` | « auto » prend la première adresse résolue ; forcez 4 ou 6 si l'hôte a les deux et que l'une ne répond pas. Choix : auto, 4, 6. |
| `loss_threshold_percent` | Perte tolérée (%) | `100` | Au-delà de ce pourcentage de paquets perdus, l'hôte est rapporté en panne. 100 : seule une perte totale compte, la perte partielle reste visible dans les graphiques. |
| `timeout_seconds` | Délai (secondes) | `5` | Budget total pour la sonde, de 1 à 60. Refusé s'il ne couvre pas « nombre d'échos × attente par paquet ». |

## TLS {#tls}

**Certificat TLS** — vérifie qu'un certificat est valide et avertit
avant son expiration, sur n'importe quel port chiffré. Exemples :
serveur mail (IMAPS, SMTPS), reverse proxy, annuaire LDAPS, courtier
MQTT, interface d'administration.

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | Poignée de main TLS (port par défaut, celui de l'adresse) |

### Configuration

1. Dans l'adresse, écrivez l'hôte et le port du service chiffré : « mail.example.com:993 » (443 si omis).

2. Si vous joignez le service par son adresse IP, saisissez le nom du certificat dans « Nom de serveur (SNI) ».

3. Pour un certificat de votre propre autorité, cochez « Accepter un certificat non vérifiable ».

!!! warning
    Pour un site web, le type « Site web ou API web » lit déjà le certificat : utilisez ce type pour les services qui ne parlent pas HTTP.

La vérification ouvre une connexion chiffrée, lit le certificat présenté et la referme aussitôt : aucune donnée n'est échangée avec l'application. Elle enregistre le nombre de jours avant l'expiration, l'émetteur et la version de TLS. Les règles d'alerte fournies avertissent à quatorze jours, puis à l'expiration.

La poignée de main se termine même quand le certificat est refusé :
c'est ce qui permet de lire la date d'expiration d'un certificat déjà
expiré. Le verdict de la vérification est dans `probe_ssl_cert_valid`.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `server_name` | Nom de serveur (SNI) | *(vide)* | Nom de domaine annoncé au serveur et vérifié dans le certificat. Réglez-le quand l'adresse est une IP derrière un reverse proxy. |
| `insecure_tls` | Accepter un certificat non vérifiable | `false` | Une chaîne non vérifiable ne compte plus comme un échec : la date d'expiration est quand même enregistrée. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Par défaut la vérification refuse les adresses que seul l'hôte DumbMonit lui-même peut atteindre (127.0.0.1, ::1, 169.254.x.x). Activez ceci pour surveiller un service tournant sur l'hôte DumbMonit. Les adresses LAN privées (10.x, 192.168.x) sont toujours autorisées. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

## SMTP

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 587 | TCP | Soumission avec STARTTLS (par défaut) |
| 465 | TCP | Seulement avec le chiffrement TLS (SMTPS) |
| 25 | TCP | Seulement sans chiffrement (relais sur le port 25) |

**Relais mail (SMTP)** — ouvre une vraie session sur un serveur mail :
salutation, EHLO, STARTTLS, et la connexion si vous en donnez une.
Exemples : le relais SMTP de votre fournisseur, un Postfix ou msmtp
local, Proxmox Mail Gateway, le port de soumission d'un serveur mail.

### Configuration

1. Dans DumbMonit, saisissez le serveur de messagerie, par exemple smtp.example.com:2525 (le port habituel du chiffrement par défaut).

2. Choisissez le chiffrement : STARTTLS pour le port 587, TLS pour le port 465, Aucun pour un relais de votre réseau sur le port 25.

3. Facultatif : renseignez un compte, pour vérifier que le relais l'accepte toujours (un mot de passe refusé n'est pas signalé comme une panne).

4. Facultatif : nommez une Extension attendue, comme STARTTLS, pour être averti quand le relais cesse de l'annoncer.

#### Droits requis

- **A mail account that may log in (AUTH)** : Seulement si vous renseignez un identifiant

!!! warning
    Une session acceptée ne prouve pas que le courrier part : surveillez les files du relais pour cela.

#### Pourquoi ces droits

Rien n'est jamais envoyé : la vérification s'arrête après le message d'accueil, les extensions et la connexion facultative, puis raccroche. Aucun message n'entre dans la file. Un relais qui cesse d'annoncer STARTTLS enverrait votre courrier en clair. Un relais qui accepte tout et le garde indéfiniment en file répond parfaitement ici.

Identifiants : aucun, ou un nom d'utilisateur et mot de passe envoyés
avec `AUTH PLAIN` (ou `AUTH LOGIN` quand le serveur n'offre que ça). Ils
sont envoyés **après** que la connexion soit chiffrée et après que le
certificat ait été jugé : un relais dont la chaîne ne peut pas être
vérifiée ne les reçoit jamais.

En TLS ou STARTTLS, le certificat est lu comme la vérification TLS le
lit — expiration, émetteur, version de protocole — un relais mail est
donc aussi un certificat sur lequel vous êtes averti.

### Ce qu'elle ne peut pas détecter

Que le mail part vraiment. Un relais qui accepte chaque session et
diffère ensuite tout répond parfaitement ici ; surveillez ses files à la
place (voir la page Proxmox Mail Gateway). Il n'envoie non plus jamais de
message, donc il ne dit rien du filtrage de destinataire, SPF ou DKIM.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `security` | Chiffrement | `starttls` | STARTTLS démarre en texte clair et monte en niveau (port 587), TLS chiffre dès le premier octet (port 465), Aucun ne chiffre jamais (port 25) et ne convient qu'à un relais sur votre propre réseau. |
| `port` | Port | *(vide)* | Utilisé si l'adresse n'en donne pas. Vide : 587 avec STARTTLS, 465 avec TLS, 25 sans chiffrement. |
| `helo_name` | Nom annoncé (EHLO) | `dumbmonit` | Nom que la vérification donne en se présentant. Un relais strict refuse un nom qu'il ne peut pas résoudre. |
| `expect_capability` | Extension attendue | *(vide)* | Extension que le serveur doit annoncer dans sa réponse EHLO, par exemple STARTTLS ou AUTH. Vide : aucune attente. |
| `server_name` | Nom de serveur (SNI) | *(l'hôte)* | Nom de domaine annoncé au serveur et vérifié dans le certificat. Réglez-le quand l'adresse est une IP. |
| `insecure_tls` | Accepter un certificat non vérifiable | `false` | Une chaîne non vérifiable ne compte plus comme un échec : la date d'expiration est quand même enregistrée. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Voir « Adresses que les vérifications refusent » ci-dessous. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Raisons d'échec

| `reason` | Ce qui s'est passé |
|---|---|
| `connect` | Le port est fermé ou le relais est inaccessible. |
| `protocol` | Quelque chose a répondu, mais pas SMTP — ou il a refusé la connexion, `EHLO`, ou le STARTTLS qu'il avait annoncé. |
| `auth` | Le compte a été refusé (`535`, `530`, `534`). Le relais va bien ; l'identifiant non. |
| `payload` | L'extension nommée dans « Extension attendue » n'est pas annoncée. |
| `tls`, `cert_expired` | Même signification que pour la vérification TLS. |

## PostgreSQL

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 5432 | TCP | PostgreSQL (port par défaut) |

**Base de données PostgreSQL** — se connecte, s'authentifie et lance une
requête : le signal d'une base de données qui tourne mais ne répond
plus. Exemples : la base derrière Nextcloud ou Immich, un recorder Home
Assistant, une base d'application, une instance TimescaleDB.

### Configuration

1. Dans psql, créez un rôle qui ne peut que se connecter.

    ```
    CREATE ROLE dumbmonit LOGIN PASSWORD 'a-long-password';
    ```

2. Autorisez-le à se connecter depuis l'hôte DumbMonit avec cette ligne dans pg_hba.conf (adaptez l'adresse), puis rechargez PostgreSQL.

    ```
    host all dumbmonit 10.0.0.5/32 scram-sha-256
    ```

3. Dans DumbMonit, saisissez le serveur, par exemple db.home.lan:5433 (port 5432 par défaut), puis l'utilisateur et le mot de passe du rôle.

4. Facultatif : remplacez la requête par défaut SELECT 1 par une requête qui renvoie un seul nombre, pour le tracer.

#### Droits requis

- **LOGIN (role attribute)** : Se connecter seulement, aucun autre privilège

!!! warning
    La vérification ouvre une vraie connexion à chaque interrogation : sur une instance proche de sa limite de connexions, donnez-lui un intervalle plus long.

#### Pourquoi ces droits

La requête par défaut est SELECT 1, qui ne lit aucune table : rien d'autre n'est à accorder. Le temps de connexion et le temps de requête sont enregistrés séparément. Une requête de votre cru qui lit une table demande SELECT sur cette table.

Identifiants : un nom d'utilisateur et mot de passe, requis. Une base de
données n'est jamais surveillée anonymement, le formulaire n'offre donc
rien d'autre.

### Ce qu'elle ne peut pas détecter

Un retard de réplication, un verrou qui s'étire, une table qui ne cesse
de grandir. Tout ça vit dans les propres vues d'administration du
moteur — écrivez la requête vous-même, et le nombre qu'elle renvoie
devient `probe_sql_value`, un graphique sur lequel vous pouvez alerter.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `port` | Port | `5432` | Utilisé si l'adresse n'en donne pas. |
| `database` | Base de données | `postgres` | Base ouverte à la connexion. Le compte de surveillance doit être autorisé à s'y connecter. |
| `query` | Requête | `SELECT 1` | Lancée à chaque vérification. La requête par défaut ne lit aucune table, le compte n'a donc besoin d'aucun privilège au-delà de se connecter. Une requête renvoyant un nombre en fait un graphique. |
| `expect` | Valeur attendue | *(vide)* | Valeur que la première colonne de la première ligne doit avoir. Vide : seul le succès de la requête est vérifié. |
| `sslmode` | Chiffrement | `prefer` | `prefer` chiffre quand le serveur le propose, `require` refuse de se connecter sans ça, `verify-full` vérifie aussi le certificat et le nom, `disable` ne chiffre jamais. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Voir « Adresses que les vérifications refusent » ci-dessous. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

## MySQL et MariaDB

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 3306 | TCP | MySQL ou MariaDB (port par défaut) |

**Base de données MySQL ou MariaDB** — se connecte, s'authentifie et
lance une requête : le signal d'une base de données qui tourne mais ne
répond plus. Exemples : la base derrière un WordPress, un Nextcloud ou un
Kimai, une base d'application, un MariaDB en Docker.

### Configuration

1. Dans le client mysql, créez un compte sans aucun privilège (adaptez la partie hôte à la machine DumbMonit).

    ```
    CREATE USER 'dumbmonit'@'%' IDENTIFIED BY 'a-long-password';
    ```

2. Dans DumbMonit, saisissez le serveur, par exemple db.home.lan:3307 (port 3306 par défaut), puis l'utilisateur et le mot de passe du compte.

3. Facultatif : remplacez la requête par défaut SELECT 1 par une requête qui renvoie un seul nombre, pour le tracer.

#### Droits requis

- **USAGE (no privilege)** : Se connecter et lancer SELECT 1, rien de plus

!!! warning
    La vérification ouvre une vraie connexion à chaque interrogation : sur une instance proche de sa limite de connexions, donnez-lui un intervalle plus long.

#### Pourquoi ces droits

La requête par défaut est SELECT 1, qui ne lit aucune table : rien d'autre n'est à accorder. Le temps de connexion et le temps de requête sont enregistrés séparément. Un motif d'hôte localhost n'accepterait que les connexions faites sur le serveur lui-même. Une requête de votre cru qui lit une table demande SELECT sur cette table.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `port` | Port | `3306` | Utilisé si l'adresse n'en donne pas. |
| `database` | Base de données | *(vide)* | Base ouverte à la connexion. Vide : aucune, ce qui suffit pour la requête par défaut. |
| `query` | Requête | `SELECT 1` | Lancée à chaque vérification. La requête par défaut ne lit aucune table, le compte n'a donc besoin d'aucun privilège au-delà de se connecter. Une requête renvoyant un nombre en fait un graphique. |
| `expect` | Valeur attendue | *(vide)* | Valeur que la première colonne de la première ligne doit avoir. Vide : seul le succès de la requête est vérifié. |
| `sslmode` | Chiffrement | `prefer` | `prefer` chiffre quand le serveur le propose, `require` refuse de se connecter sans ça, `verify-full` vérifie aussi le certificat et le nom, `disable` ne chiffre jamais. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Voir « Adresses que les vérifications refusent » ci-dessous. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Raisons d'échec, les deux moteurs

| `reason` | Ce qui s'est passé |
|---|---|
| `connect` | Le port est fermé, le serveur est inaccessible, ou il a refusé la connexion pour une raison autre que le compte (démarrage, trop de connexions). |
| `auth` | Le compte a été refusé : mauvais mot de passe, aucune ligne `pg_hba.conf`, motif d'hôte qui ne couvre pas la machine DumbMonit. |
| `query` | Connecté et authentifié, mais la requête a échoué : table manquante, base en lecture seule, erreur de syntaxe. |
| `payload` | La requête a fonctionné mais a renvoyé autre chose que la valeur attendue. |
| `timeout` | La connexion ou la requête n'a pas fini à temps — le cas « actif mais lent ». |

## MQTT

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 1883 | TCP | MQTT (port par défaut) |
| 8883 | TCP | Seulement avec l'option Connexion chiffrée (MQTT sur TLS, par défaut) |

**Courtier MQTT** — se connecte au courtier, s'abonne à un topic, et
peut attendre un message retenu. Exemples : Mosquitto, le courtier
derrière Home Assistant, Zigbee2MQTT, les capteurs ESPHome.

### Configuration

1. Dans DumbMonit, saisissez le courtier, par exemple broker.home.lan:1884 (port 1883 par défaut).

2. Cochez Connexion chiffrée pour un courtier qui écoute sur 8883.

3. Si le courtier exige un compte, renseignez son utilisateur et son mot de passe.

4. Facultatif : nommez un topic, pour vérifier que le compte peut toujours s'y abonner.

5. Facultatif : cochez Attendre un message retenu seulement pour un topic qui en porte un, typiquement un topic de disponibilité.

#### Droits requis

- **Subscribe (read) on the topic you name** : Seulement si vous nommez un topic

!!! warning
    La vérification ne publie jamais, et ne peut pas savoir l'âge d'un message retenu : MQTT ne les date pas.

#### Pourquoi ces droits

Se connecter vaut déjà la peine d'être surveillé : un courtier qui refuse votre compte, ou qui ne répond plus, est la raison pour laquelle toute la maison s'est tue. Quand vous nommez un topic, un courtier qui refuse l'abonnement vous dit que le compte a perdu ses droits d'accès. Un topic sur lequel on publie de temps en temps paraît silencieux à un client qui vient de se connecter : n'attendez un message retenu que pour un topic qui en porte un. Une valeur figée il y a trois semaines compte toujours comme présente.

### Ce qu'elle ne peut pas détecter

Qu'un capteur donné a arrêté d'émettre. Le message retenu que la
vérification lit est ce que le courtier a gardé, peut-être vieux de trois
semaines — MQTT ne l'horodate pas. Pour la fraîcheur, surveillez
l'automatisation qui le consomme, ou utilisez un battement de cœur.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `tls` | Connexion chiffrée | `false` | Chiffre la session dès le premier octet, comme les courtiers le font sur le port 8883. |
| `port` | Port | *(vide)* | Utilisé si l'adresse n'en donne pas. Vide : 1883 en texte clair, 8883 chiffré. |
| `topic` | Topic | *(vide)* | Topic auquel la vérification s'abonne. Vide : elle se connecte seulement, ce qui vous dit déjà que le courtier est vivant et accepte votre compte. |
| `expect_message` | Attendre un message retenu | `false` | Un message doit arriver sur ce topic avant le délai. Seuls les messages retenus arrivent tout de suite : un topic juste publié de temps en temps paraîtra silencieux. |
| `expect` | Contenu attendu | *(vide)* | Texte que ce message doit contenir. Le remplir implique d'attendre un message. |
| `client_id` | Identifiant client | *(dérivé)* | Nom annoncé au courtier. Le défaut dérive de la surveillance, deux vérifications ne se déconnectent donc jamais l'une l'autre. |
| `server_name` | Nom de serveur (SNI) | *(l'hôte)* | Nom de domaine annoncé au courtier et vérifié dans le certificat. Réglez-le quand l'adresse est une IP. |
| `insecure_tls` | Accepter un certificat non vérifiable | `false` | Une chaîne non vérifiable ne compte plus comme un échec : la date d'expiration est quand même enregistrée. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Voir « Adresses que les vérifications refusent » ci-dessous. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Raisons d'échec

| `reason` | Ce qui s'est passé |
|---|---|
| `connect` | Le port est fermé ou le courtier est inaccessible. |
| `auth` | Le `CONNACK` a refusé le compte, ou le courtier a refusé l'abonnement — une liste de contrôle d'accès ne couvre plus ce topic. |
| `protocol` | Quelque chose a répondu mais ne parle pas MQTT 3.1.1, ou le courtier a rejeté l'identifiant client. |
| `payload` | Aucun message retenu n'est arrivé sur le topic, ou son contenu ne contient pas le texte attendu. |

## WebSocket

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | WSS (par défaut) |
| 80 | TCP | Seulement avec une adresse ws:// |

**Point de terminaison WebSocket** — lance la poignée de main de mise à
niveau, et peut envoyer une trame et en attendre une : une panne
différente d'une simple page HTTP. Exemples : l'API Home Assistant, un
tableau de bord en direct, un flux de logs, un point de terminaison
derrière un reverse proxy.

### Configuration

1. Dans DumbMonit, collez le point de terminaison complet, par exemple wss://home.example.com/api/websocket (wss est supposé sans schéma).

2. Facultatif : renseignez Contenu attendu avec un fragment de la première trame que le serveur envoie de lui-même.

3. Facultatif : si le point de terminaison ne dit rien tant qu'on ne lui parle pas, mettez un message dans Trame à envoyer, et ce que la réponse doit contenir dans Contenu attendu.

!!! warning
    La vérification raccroche en moins d'une seconde : une connexion qu'un pare-feu ou un proxy coupe après trente secondes paraît saine ici.

#### Pour aller plus loin

La vérification déroule toute la poignée de main d'ouverture et vérifie la réponse que le serveur calcule à partir de la clé reçue : un reverse proxy qui a perdu l'en-tête Upgrade est repéré ici, là où une vérification HTTP verrait encore un 200 sain. Un statut autre que 101 est rapporté avec son code, donc un 401 dû à un jeton expiré n'est pas pris pour une panne. Pour la première trame, pensez au message d'accueil de l'API Home Assistant.

Identifiants : aucun, un nom d'utilisateur et mot de passe (envoyés en
authentification HTTP basique dans la requête d'ouverture) ou un jeton
(envoyé comme `Authorization: Bearer`).

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `path` | Chemin | *(depuis l'adresse)* | Chemin de la requête d'ouverture. Pris depuis l'adresse quand elle en porte un. |
| `port` | Port | *(vide)* | Utilisé si l'adresse n'en donne pas. Vide : 443 pour wss, 80 pour ws. |
| `send` | Trame à envoyer | *(vide)* | Trame texte envoyée une fois la connexion ouverte. Vide : rien n'est envoyé. |
| `expect` | Contenu attendu | *(vide)* | Texte qu'une trame reçue doit contenir. Vide et sans rien à envoyer, seule la poignée de main d'ouverture est vérifiée. |
| `subprotocol` | Sous-protocole | *(vide)* | Valeur de Sec-WebSocket-Protocol. La vérification échoue si le serveur en choisit un autre. |
| `origin` | Origine | *(vide)* | Valeur de l'en-tête Origin, que certains serveurs exigent avant la mise à niveau. |
| `server_name` | Nom de serveur (SNI) | *(l'hôte)* | Nom de domaine annoncé au serveur et vérifié dans le certificat. Réglez-le quand l'adresse est une IP. |
| `insecure_tls` | Accepter un certificat non vérifiable | `false` | Une chaîne non vérifiable ne compte plus comme un échec : la date d'expiration est quand même enregistrée. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Voir « Adresses que les vérifications refusent » ci-dessous. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Raisons d'échec

| `reason` | Ce qui s'est passé |
|---|---|
| `connect` | Le port est fermé ou l'hôte est inaccessible. |
| `status` | Le point de terminaison a répondu autre chose que `101` ; `probe_http_status_code` porte le code. |
| `protocol` | Il a répondu `101` mais son `Sec-WebSocket-Accept` ne correspond pas à la clé envoyée, il a choisi un autre sous-protocole, ou il a fermé la connexion tout de suite. |
| `payload` | Aucune trame n'est arrivée, ou elle ne contient pas le texte attendu. |

## NTP {#ntp}

**Serveur de temps NTP** — envoie une seule requête SNTP et lit la
réponse : décalage, délai d'aller-retour, stratum et indicateur de
saut. Exemples : un routeur ou NAS servant de serveur de temps local, un
serveur NTP interne (chrony, ntpd), un serveur de pool public utilisé
comme référence.

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 123 | UDP | Requête NTP (port par défaut) |

### Configuration

1. Dans l'adresse, écrivez le nom ou l'adresse IP du serveur : « pool.ntp.org » ou « 192.168.1.1 ».

2. Baissez « Décalage toléré » dans les options pour être averti avant 100 ms.

!!! warning
    Gardez l'hôte DumbMonit lui-même synchronisé : sa propre dérive s'ajoute au décalage mesuré.

Le port UDP 123 est utilisé sauf indication contraire de l'option « Port ». La vérification envoie une seule requête SNTP et enregistre le décalage, le délai aller-retour, la strate et l'indicateur de saut de seconde. Un serveur en strate 16, ou dont l'indicateur de saut de seconde indique « non synchronisé », est signalé en panne quel que soit le décalage.

Identifiants : aucun.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `port` | Port | `123` | Port UDP du serveur NTP. |
| `offset_threshold_ms` | Décalage toléré (ms) | `100` | Au-delà de ce décalage d'horloge, le serveur est rapporté en panne. Un serveur non synchronisé (stratum 16) échoue toujours, quelle que soit cette valeur. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Voir « Adresses que les vérifications refusent » ci-dessous. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Raisons d'échec

| `reason` | Ce qui s'est passé |
|---|---|
| `connect` | La socket UDP n'a pas pu être ouverte, ou la réponse venait d'une adresse inattendue. |
| `timeout` | Aucune réponse n'est arrivée dans le délai. |
| `protocol` | La réponse n'était pas un paquet NTP valide, ou ne venait pas d'un serveur (mauvais mode). |
| `unsynchronized` | Le serveur a répondu stratum 16 ou 0 (un « kiss-o'-death », qui nomme aussi la raison dans le journal), ou son indicateur de saut lit « non synchronisé ». |
| `clock_offset` | Le décalage mesuré dépasse « Décalage toléré ». |

## UDP {#udp}

**Service UDP** — envoie un datagramme à un port UDP et vérifie la réponse, ou
que personne ne le refuse. Exemples : un serveur de jeu ou de VoIP, un
récepteur syslog ou SNMP, un point d'accès WireGuard, un service maison qui
répond à un message de ping.

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 161 | UDP | Le port UDP du service, tel qu'écrit dans l'adresse (161 n'est qu'un exemple) |

### Configuration

1. Dans l'adresse, écrivez l'hôte et le port UDP : « 192.168.1.20:5060 ». Le port est obligatoire.

2. Pour vérifier que le service répond vraiment, renseignez « Datagramme à envoyer » et « Réponse attendue ».

3. Sans réponse attendue, la vérification n'échoue que si l'hôte répond que le port est fermé.

!!! warning
    UDP n'a pas de connexion : sans réponse attendue, un pare-feu qui jette les datagrammes en silence paraît en bonne santé.

Il n'y a pas de port par défaut. Avec une réponse attendue, la vérification
attend un datagramme en retour pendant tout le délai et y cherche le texte ;
sans, elle attend environ une seconde et demie que l'hôte réponde par un ICMP
« port unreachable » (rapporté comme `connect`). Les datagrammes en texte et
en hexadécimal sont pris en charge ; en texte, `\n`, `\r`, `\t`, `\0`, `\\` et
`\xNN` sont lus comme des échappements. La réponse est cherchée comme du texte
brut : il n'y a pas de forme en expression régulière.

Identifiants : aucun.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `port` | Port | *(vide)* | Utilisé si l'adresse n'en donne pas. Il n'y a pas de défaut : UDP n'a pas de port standard. |
| `payload` | Datagramme à envoyer | *(vide)* | Envoyé en un seul datagramme, 1400 octets au plus. En texte, \n \r \t \0 \\ et \xNN sont lus comme des échappements. Vide : un datagramme vide. |
| `payload_encoding` | Codage du datagramme | `text` | Text envoie les caractères tels quels, Hex lit des paires de chiffres hexadécimaux (espaces permis). |
| `expect_response` | Réponse attendue | *(vide)* | Texte que la réponse doit contenir. Vide : la vérification réussit sauf si l'hôte répond que le port est fermé, et le silence compte comme un succès. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Voir « Adresses que les vérifications refusent » ci-dessous. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Raisons d'échec

| `reason` | Ce qui s'est passé |
|---|---|
| `dns` | Le nom ne se résout pas. |
| `connect` | L'hôte a répondu que le port est fermé (ICMP), ou la socket n'a pas pu être ouverte. |
| `timeout` | Une réponse attendue n'est pas arrivée dans le délai. |
| `payload` | Une réponse est arrivée mais ne contient pas le texte attendu. |

## SSH {#ssh}

**Serveur SSH** — lit la bannière SSH et, si vous le demandez, vérifie que la
clé d'hôte est toujours celle que vous connaissez. Exemples : un serveur Linux
ou un Raspberry Pi, un NAS ou un hyperviseur avec SSH activé, un bastion, un
serveur Git en SSH.

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 22 | TCP | SSH (port par défaut, celui de l'adresse) |

### Configuration

1. Dans l'adresse, écrivez le nom d'hôte ou l'adresse IP, avec le port s'il n'est pas 22 : « server.home.lan:2222 ».

2. Pour être averti quand l'identité du serveur change, lisez son empreinte avec ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub.

3. Collez cette empreinte (SHA256:...) dans « Clé d'hôte attendue », ou cochez « Alerter quand la clé d'hôte change » pour l'enregistrer sans en choisir une.

!!! warning
    La vérification ne se connecte jamais et ne contrôle pas la signature du serveur : elle lit la clé que le serveur montre, comme ssh-keyscan.

La vérification lit toujours la bannière (`SSH-2.0-OpenSSH_9.6p1 ...`) : la
version du protocole et le logiciel du serveur sont enregistrés en labels.
Quand une clé d'hôte attendue est renseignée, ou que « Alerter quand la clé
d'hôte change » est cochée, elle démarre aussi l'échange de clés
(`curve25519-sha256`) assez loin pour que le serveur montre sa clé publique
d'hôte, en calcule l'empreinte SHA-256 et raccroche. Aucun identifiant n'est
jamais envoyé.

Un serveur qui détient plusieurs clés d'hôte montre la première qu'il possède
parmi `ssh-ed25519`, `ecdsa-sha2-nistp256`, `rsa-sha2-512` et `rsa-sha2-256`.
L'empreinte attendue doit être celle de cette clé : utilisez « Type de clé
d'hôte » pour choisir la famille quand vous la lisez dans un autre fichier que
celui d'ed25519.

Identifiants : aucun.

### Alerter quand la clé d'hôte change

Une clé d'hôte qui change sans que vous ayez réinstallé la machine mérite un
coup d'œil : un serveur reconstruit puis restauré sans ses clés, ou quelque
chose qui répond à sa place. Avec une clé d'hôte attendue, la vérification
échoue avec `reason="host_key"` et la règle Service en panne se déclenche après
trois minutes. Sans, cochez « Alerter quand la clé d'hôte change » et créez une
règle sur l'empreinte, dans **Alertes > Règles** :

```
changes_prometheus(dumbmonit_probe_ssh_host_key_fingerprint[1h]) > 0
```

Elle se déclenche quand l'empreinte diffère de la précédente et se résout
d'elle-même une heure plus tard, comme la règle intégrée de changement de
réponse DNS. L'empreinte lisible est dans `probe_ssh_host_key_info`.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `port` | Port | `22` | Utilisé si l'adresse n'en donne pas. |
| `expected_host_key` | Clé d'hôte attendue | *(vide)* | Empreinte SHA256 de la clé d'hôte du serveur, telle qu'affichée par ssh-keygen -lf. Si la clé diffère, le serveur est rapporté en panne avec la raison host_key. Vide : la clé n'est pas vérifiée. |
| `alert_on_change` | Alerter quand la clé d'hôte change | `false` | Enregistre une empreinte de la clé d'hôte à chaque vérification, pour qu'une règle puisse se déclencher quand elle change. Implicite avec une clé d'hôte attendue. |
| `host_key_type` | Type de clé d'hôte | `auto` | Un serveur peut détenir plusieurs clés d'hôte. Auto lit la première qu'il propose dans cet ordre : ed25519, ecdsa, rsa. Choisissez la famille de l'empreinte attendue. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Voir « Adresses que les vérifications refusent » ci-dessous. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Raisons d'échec

| `reason` | Ce qui s'est passé |
|---|---|
| `dns`, `connect`, `timeout` | Le nom ne se résout pas, le port est fermé, ou le serveur n'a pas répondu à temps. |
| `protocol` | Quelque chose a répondu, mais pas du SSH 2.0 — ou le serveur ne propose pas `curve25519-sha256` ni aucun des types de clé d'hôte demandés, ou il s'est déconnecté pendant l'échange de clés. |
| `host_key` | La clé d'hôte ne correspond pas à « Clé d'hôte attendue ». Le journal nomme le type de clé et les deux empreintes. |

## IMAP {#imap}

**Boîte aux lettres (IMAP)** — un serveur IMAP salue, propose TLS et accepte une
connexion, comme un vrai client de messagerie. Exemples : Dovecot, le serveur
IMAP de votre fournisseur, une boîte derrière Mailcow ou Mailu, l'hôte mail de
Proxmox Mail Gateway.

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 993 | TCP | IMAP sur TLS (par défaut) |
| 143 | TCP | Seulement avec STARTTLS ou sans chiffrement |

### Configuration

1. Dans DumbMonit, saisissez le serveur de messagerie, par exemple imap.example.com:1993 (port 993 avec le chiffrement par défaut).

2. Choisissez le chiffrement : TLS pour le port 993, STARTTLS pour le port 143, Aucun pour un serveur de votre propre réseau.

3. Facultatif : renseignez un compte de boîte aux lettres, pour vérifier que le serveur accepte toujours les connexions (un mot de passe refusé n'est pas rapporté comme une panne).

#### Droits requis

- **Un compte de boîte aux lettres autorisé à se connecter** : seulement si vous renseignez un identifiant

!!! warning
    La vérification n'ouvre jamais de boîte : un serveur qui accepte la connexion mais ne sait plus lire son stockage répond parfaitement ici.

La vérification lit la bannière `* OK`, chiffre la connexion si demandé
(`STARTTLS`), envoie `LOGIN` si la cible a un identifiant, puis `LOGOUT`. Elle
ne sélectionne jamais de boîte et ne lit aucun message.

Identifiants : aucun, ou un nom d'utilisateur et un mot de passe envoyés avec
`LOGIN`. Ils sont envoyés **après** le chiffrement de la connexion et le
jugement du certificat : un serveur dont la chaîne ne peut pas être vérifiée ne
les reçoit jamais. Seule exception, le chiffrement « Aucun », que vous devez
choisir explicitement et qui les envoie en clair.

En TLS ou STARTTLS, le certificat est lu comme le fait la vérification TLS : une
boîte aux lettres est donc aussi un certificat dont vous êtes averti.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `security` | Chiffrement | `tls` | TLS chiffre dès le premier octet (port 993), STARTTLS démarre en clair puis passe en chiffré (port 143), Aucun ne chiffre jamais (port 143) et envoie le mot de passe en clair : seulement pour votre propre réseau. |
| `port` | Port | *(vide)* | Utilisé si l'adresse n'en donne pas. Vide : 993 avec TLS, 143 avec STARTTLS ou sans chiffrement. |
| `server_name` | Nom de serveur (SNI) | *(l'hôte)* | Nom de domaine annoncé au serveur et vérifié dans le certificat. Réglez-le quand l'adresse est une IP. |
| `insecure_tls` | Accepter un certificat non vérifiable | `false` | Une chaîne non vérifiable ne compte plus comme un échec : la date d'expiration est quand même enregistrée. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Voir « Adresses que les vérifications refusent » ci-dessous. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Raisons d'échec

| `reason` | Ce qui s'est passé |
|---|---|
| `connect` | Le port est fermé ou le serveur est injoignable. |
| `protocol` | Quelque chose a répondu, mais pas de l'IMAP — ou le serveur a raccroché, ou a répondu `BAD` ou `[UNAVAILABLE]` à la connexion. |
| `auth` | Le compte a été refusé (`NO`). Le serveur va bien ; l'identifiant, non. |
| `tls`, `cert_expired` | Même sens que pour la vérification TLS. Un `STARTTLS` refusé est rapporté comme `tls`. |

## POP3 {#pop3}

**Boîte aux lettres (POP3)** — un serveur POP3 salue, propose TLS et accepte une
connexion, comme un vrai client de messagerie. Exemples : Dovecot, le serveur
POP3 de votre fournisseur, un vieux serveur de messagerie qui ne parle que
POP3.

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 995 | TCP | POP3 sur TLS (par défaut) |
| 110 | TCP | Seulement avec STARTTLS ou sans chiffrement |

### Configuration

1. Dans DumbMonit, saisissez le serveur de messagerie, par exemple pop.example.com:1995 (port 995 avec le chiffrement par défaut).

2. Choisissez le chiffrement : TLS pour le port 995, STARTTLS pour le port 110 (commande STLS), Aucun pour un serveur de votre propre réseau.

3. Facultatif : renseignez un compte de boîte aux lettres, pour vérifier que le serveur accepte toujours les connexions (un mot de passe refusé n'est pas rapporté comme une panne).

#### Droits requis

- **Un compte de boîte aux lettres autorisé à se connecter** : seulement si vous renseignez un identifiant

!!! warning
    La vérification ne liste ni ne lit jamais de messages : elle se connecte, puis prend congé.

La vérification lit la bannière `+OK`, chiffre la connexion si demandé (`STLS`),
envoie `USER` et `PASS` si la cible a un identifiant, puis `QUIT`. Ni `STAT` ni
`RETR` ne sont jamais envoyés, et rien n'est supprimé.

Identifiants : aucun, ou un nom d'utilisateur et un mot de passe. Même règle
qu'en IMAP : ils sont envoyés après le chiffrement de la connexion et le
jugement du certificat, sauf si vous avez explicitement choisi le chiffrement
« Aucun ».

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `security` | Chiffrement | `tls` | TLS chiffre dès le premier octet (port 995), STARTTLS démarre en clair puis passe en chiffré avec STLS (port 110), Aucun ne chiffre jamais (port 110) et envoie le mot de passe en clair : seulement pour votre propre réseau. |
| `port` | Port | *(vide)* | Utilisé si l'adresse n'en donne pas. Vide : 995 avec TLS, 110 avec STARTTLS ou sans chiffrement. |
| `server_name` | Nom de serveur (SNI) | *(l'hôte)* | Nom de domaine annoncé au serveur et vérifié dans le certificat. Réglez-le quand l'adresse est une IP. |
| `insecure_tls` | Accepter un certificat non vérifiable | `false` | Une chaîne non vérifiable ne compte plus comme un échec : la date d'expiration est quand même enregistrée. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Voir « Adresses que les vérifications refusent » ci-dessous. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Raisons d'échec

| `reason` | Ce qui s'est passé |
|---|---|
| `connect` | Le port est fermé ou le serveur est injoignable. |
| `protocol` | Quelque chose a répondu, mais pas du POP3 — ou la boîte est verrouillée (`[IN-USE]`) ou le système est en difficulté (`[SYS/...]`). |
| `auth` | `USER` ou `PASS` a été refusé (`-ERR`). Le serveur va bien ; l'identifiant, non. |
| `tls`, `cert_expired` | Même sens que pour la vérification TLS. Un `STLS` refusé est rapporté comme `tls`. |

## SNMP OID {#snmp-oid}

**Valeur SNMP (OID unique)** — lit un objet SNMP et compare sa valeur à une
plage ou à un texte. Exemples : une sonde de température ou d'humidité, l'état
ou la charge d'un onduleur, une chaîne de version ou un indicateur de firmware,
un compteur exposé par un script `extend` de Net-SNMP.

Là où le type d'équipement SNMP décrit toute une machine par un profil, celui-ci
pose une question étroite : cet objet contient-il ce que j'attends ?

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 161 | UDP | Requête SNMP (port par défaut) |

### Configuration

1. Sur l'équipement, activez SNMP et vérifiez que la communauté ou l'utilisateur v3 peut lire l'objet (la vue SNMP doit donner accès à sa branche).

2. Dans DumbMonit, saisissez l'adresse de l'équipement, puis la communauté ou l'utilisateur v3.

3. Renseignez l'objet (OID) avec son suffixe d'instance, par exemple 1.3.6.1.2.1.1.3.0.

4. Facultatif : fixez un minimum et un maximum, ou un texte attendu, pour être averti quand la valeur dérive.

#### Droits requis

- **Accès en lecture à l'objet** : Une communauté ou un utilisateur v3 en lecture seule suffit

!!! warning
    Un objet que l'agent ne connaît pas, ou que sa vue SNMP masque, est rapporté en panne avec la raison not_found.

#### À savoir

La vérification réutilise la session SNMP du type d'équipement SNMP : mêmes communautés (v1/v2c) et utilisateurs v3, mêmes renvois. Sans borne ni texte attendu, elle réussit dès que l'objet existe. Une valeur numérique est tracée dans `probe_snmp_value` (un texte qui contient un nombre, comme `42,5`, compte aussi). Les bornes sont incluses ; une valeur qui n'est pas un nombre échoue dès qu'une borne est posée. Le texte attendu est cherché sans tenir compte de la casse dans la valeur lue comme du texte.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `oid` | Objet (OID) | *(vide)* | L'objet SNMP à lire, sous forme numérique. Ajoutez le suffixe d'instance, souvent .0 pour une valeur simple. |
| `port` | Port | `161` | Utilisé si l'adresse n'en donne pas. |
| `min` | Valeur minimale | *(vide)* | Une valeur numérique inférieure échoue. Vide : pas de borne basse. |
| `max` | Valeur maximale | *(vide)* | Une valeur numérique supérieure échoue. Vide : pas de borne haute. |
| `expect` | Texte attendu | *(vide)* | Texte que la valeur doit contenir, quelle que soit la casse. Vide : toute valeur est acceptée. |
| `snmp_version` | Version SNMP | `2c` | Version utilisée avec une communauté. Choisissez 1 seulement pour un vieil agent qui ignore la 2c ; les utilisateurs SNMP v3 l'ignorent. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Voir « Adresses que les vérifications refusent » ci-dessous. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Raisons d'échec

| `reason` | Ce qui s'est passé |
|---|---|
| `connect` | L'équipement est injoignable. |
| `timeout` | L'agent n'a pas répondu à temps : mauvaise adresse, mauvaise communauté (les agents restent muets) ou pare-feu. |
| `auth` | L'agent a refusé la communauté ou les identifiants v3. |
| `protocol` | L'agent a répondu autre chose qu'une réponse SNMP valide, ou a refusé la requête. |
| `not_found` | L'agent ne connaît pas l'objet (`noSuchObject`, `noSuchInstance`) : vérifiez l'OID, son suffixe d'instance et la vue SNMP. |
| `threshold` | La valeur numérique est hors de `min` / `max`. |
| `payload` | La valeur n'est pas un nombre alors qu'une borne est posée, ou ne contient pas le texte attendu. |

## RADIUS {#radius}

**Serveur RADIUS** — authentifie un compte de test (Access-Request) et attend
un Access-Accept : il détecte un serveur allumé qui n'accepte plus les
connexions. Exemples : FreeRADIUS, Windows NPS, le RADIUS d'un Wi-Fi
WPA-Enterprise, d'un VPN ou d'un switch 802.1X.

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 1812 | UDP | Requêtes d'authentification (port par défaut) |

### Configuration

1. Sur le serveur RADIUS, déclarez l'hôte DumbMonit comme client, avec un secret partagé (clients.conf sur FreeRADIUS, RADIUS Clients sur Windows NPS).

2. Créez un compte de test sans second facteur, et notez son nom et son mot de passe.

3. Dans DumbMonit, saisissez le serveur (port UDP 1812 par défaut), le secret partagé et le compte de test.

4. Facultatif : changez l'identifiant NAS si le serveur filtre les requêtes dessus.

#### Droits requis

- **Un compte de test autorisé à s'authentifier** : Aucun accès réseau ni droit d'administration n'est nécessaire

!!! warning
    Un secret partagé erroné, ou un hôte non déclaré comme client, rend généralement le serveur muet : la vérification signale alors un délai dépassé.

#### À savoir

La vérification envoie un Access-Request par passage, avec le nom du compte, son mot de passe masqué par le secret partagé (RFC 2865) et une signature (`Message-Authenticator`, RFC 3579, placée en premier comme le demande la parade à BlastRADIUS). Elle ne réessaie jamais : un datagramme perdu apparaît comme une panne.

Le secret partagé et le mot de passe sont stockés chiffrés et ne ressortent jamais par l'API. Les laisser vides en modifiant la surveillance conserve ceux qui sont enregistrés ; changer le nom du compte redemande son mot de passe.

La réponse est vérifiée avec le secret partagé. Un serveur dont la signature ne correspond pas est signalé `bad_secret` plutôt que comme une panne, pour que l'alerte dise quoi corriger. Un serveur qui ne connaît pas le secret ne répond en général pas du tout : voir les raisons d'échec.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `port` | Port | `1812` | Port UDP du service d'authentification. |
| `nas_identifier` | Identifiant NAS | `dumbmonit` | Valeur de l'attribut NAS-Identifier envoyé avec la requête. Changez-la si le serveur filtre les requêtes par NAS. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Voir « Adresses que les vérifications refusent » ci-dessous. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Raisons d'échec

| `reason` | Ce qui s'est passé |
|---|---|
| `connect` | La socket UDP n'a pas pu être ouverte. |
| `timeout` | Aucune réponse valide à temps : serveur arrêté, port filtré, secret partagé erroné ou hôte non déclaré comme client (les serveurs ignorent ces requêtes sans rien dire). |
| `auth` | Access-Reject : le serveur fonctionne et refuse le compte de test (verrouillé, expiré, mot de passe erroné). Son Reply-Message, s'il y en a un, figure dans le journal. |
| `bad_secret` | La signature de la réponse ne correspond pas : le secret partagé est probablement faux. |
| `protocol` | Access-Challenge (le compte demande une seconde étape), un autre code inattendu, ou une réponse qui n'est pas du RADIUS valide. |

## gRPC {#grpc}

**Service gRPC** — appelle le contrôle de santé gRPC standard
(`grpc.health.v1.Health/Check`) : il détecte un service allumé qui se déclare
en mauvais état. Exemples : un microservice gRPC, etcd, un service derrière
Envoy ou un maillage de services, tout serveur qui enregistre `grpc.health.v1`.

Une vérification HTTP ne peut pas le faire : gRPC exige HTTP/2, souvent sans
TLS (h2c), et la réponse est un message protobuf accompagné d'un `grpc-status`
dans les trailers.

### Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 50051 | TCP | gRPC sur HTTP/2 (port par défaut) |
| 443 | TCP | Seulement avec l'option Connexion chiffrée (port par défaut) |

### Configuration

1. Vérifiez que le serveur enregistre le service de santé standard (grpc.health.v1.Health) : la plupart des frameworks gRPC offrent une fonction d'une ligne pour cela.

2. Dans DumbMonit, saisissez le serveur avec son port, par exemple grpc.home.lan:50051.

3. Cochez Connexion chiffrée (TLS) pour un serveur qui l'exige ; le port vaut alors 443 par défaut.

4. Facultatif : nommez un Service, pour vérifier sa propre santé plutôt que celle du serveur entier.

!!! warning
    La vérification ne voit que ce que le service de santé déclare : un serveur qui répond toujours SERVING ne prouve rien sur les méthodes qu'il porte.

#### À savoir

Sans TLS, la vérification parle HTTP/2 en clair dès le premier octet (« prior knowledge »), ce qu'attendent les serveurs gRPC d'un réseau interne. Avec TLS, elle propose l'ALPN `h2` et échoue en `protocol` si le serveur ne l'accepte pas, et elle lit le certificat comme les autres vérifications chiffrées (expiration, émetteur, « Accepter un certificat invérifiable »).

La réponse est jugée en deux temps. D'abord le `grpc-status`, lu dans les trailers ou, pour une réponse « Trailers-Only », dans les en-têtes : tout sauf `OK` échoue. Ensuite le champ `status` du message de santé : seul `SERVING` réussit.

Identifiants : aucun.

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `service` | Service | *(vide)* | Nom du service dont on demande la santé, par exemple my.package.MyService. Vide : la santé du serveur dans son ensemble. |
| `tls` | Connexion chiffrée (TLS) | `false` | Se connecter en TLS et HTTP/2 (ALPN h2). Décochée, la vérification parle HTTP/2 en clair (h2c), comme la plupart des services d'un réseau interne. |
| `port` | Port | *(vide)* | Utilisé si l'adresse n'en donne pas. Vide : 50051, ou 443 avec TLS. |
| `server_name` | Nom du serveur (SNI) | *(l'hôte)* | Nom de domaine annoncé au serveur et vérifié dans le certificat. À renseigner quand l'adresse est une IP. |
| `insecure_tls` | Accepter un certificat invérifiable | `false` | Une chaîne invérifiable ne compte plus comme un échec : la date d'expiration reste enregistrée. |
| `allow_private_targets` | Autoriser les cibles loopback et link-local | `false` | Voir « Adresses que les vérifications refusent » ci-dessous. |
| `timeout_seconds` | Délai (secondes) | `5` | Temps après lequel le service est rapporté en panne s'il n'a pas répondu. Entre 1 et 60. |

### Raisons d'échec

| `reason` | Ce qui s'est passé |
|---|---|
| `dns`, `connect`, `timeout`, `tls`, `cert_expired` | Comme pour les autres vérifications : nom, port, délai, poignée de main TLS, certificat. |
| `unhealthy` | Le serveur répond mais déclare `NOT_SERVING`, `UNKNOWN` ou `SERVICE_UNKNOWN`, ou répond `UNAVAILABLE`. `probe_grpc_serving_status` porte la valeur. |
| `protocol` | Pas du HTTP/2, pas du gRPC, ou un `grpc-status` autre que ceux ci-dessous. `UNIMPLEMENTED` veut dire que le serveur n'expose pas le service de santé standard ; `NOT_FOUND`, qu'il ne connaît pas le nom donné dans `service`. |
| `status` | Le point de terminaison a répondu un statut HTTP autre que 200 (un proxy, un autre service) ; `probe_http_status_code` le porte. |
| `auth` | `UNAUTHENTICATED` ou `PERMISSION_DENIED` : le serveur, ou un proxy devant lui, veut des identifiants que le protocole de santé n'envoie pas. |

## Adresses que les vérifications refusent

Les vérifications qui se connectent à une adresse que vous leur donnez —
HTTP, TCP, TLS, NTP, UDP, SSH, SMTP, IMAP, POP3, PostgreSQL, MySQL, MQTT, WebSocket —
rapportent ce qu'elles voient — ce qui est aussi, mot pour mot, ce qu'une
falsification de requête côté serveur fait. Pour empêcher un
administrateur de surveillance de lire des services que seul l'hôte
DumbMonit peut atteindre (la base de métriques embarquée sur
`127.0.0.1:8428`, le service de métadonnées d'un fournisseur cloud sur
`169.254.169.254`), les vérifications refusent par défaut :

- les adresses loopback (`127.0.0.0/8`, `::1`) ;
- les adresses link-local (`169.254.0.0/16`, `fe80::/10`), incluant
  l'adresse de métadonnées cloud, et l'adresse non spécifiée
  (`0.0.0.0`, `::`).

Les plages LAN privées (`10.0.0.0/8`, `172.16.0.0/12`,
`192.168.0.0/16`, `fc00::/7`) sont toujours autorisées : les surveiller
est le but d'un homelab.

La règle s'applique à l'adresse que vous avez tapée, à chaque adresse
qu'un nom d'hôte résout (vérifiée à nouveau au moment de la connexion,
un nom qui change sa réponse ne peut donc pas passer à travers), et à
chaque redirection que la vérification HTTP suit. Une adresse refusée se
montre comme une erreur de configuration sur la cible — aucune alerte
« en panne » n'est envoyée — nommant l'option à activer :
`allow_private_targets`. Activez-la pour une cible qui surveille
vraiment un service sur l'hôte DumbMonit lui-même.

Une redirection vers une adresse refusée est rapportée comme une mesure
échouée (`reason="connect"`), parce que c'est le service surveillé qui a
changé, pas la configuration.

Les valeurs trouvées à un `json_path` ne sont jamais répétées au-delà de
32 caractères dans un message d'échec.

## Erreurs courantes

| Symptôme | Cause probable |
|---|---|
| Ping montre une erreur de configuration | Le sysctl `net.ipv4.ping_group_range` manque du fichier Compose. Voir ci-dessus. |
| HTTP en panne avec `reason="keyword"` | Le mot-clé est cherché seulement dans les premiers `max_body_bytes`, sans tenir compte de la casse sauf si `keyword_case_sensitive` est réglé. |
| HTTP en panne avec `reason="tls"` | Autosigné ou autorité privée : réglez `insecure_tls`. |
| Une vérification montre une erreur de configuration nommant `allow_private_targets` | L'adresse est loopback ou link-local. Voir « Adresses que les vérifications refusent » ci-dessus. |
| TCP en panne alors que `telnet host port` fonctionne depuis votre portable | Le conteneur DumbMonit ne peut pas atteindre l'hôte (réseau Docker, pare-feu). Testez depuis l'intérieur du réseau du conteneur. |
| DNS en panne avec `reason="record"` | La réponse ne contient pas chaque valeur listée dans `expect`. |
| Ping en panne avec `reason="packet_loss"` | Perte au-dessus de `loss_threshold_percent` ; certains appareils limitent le débit ICMP. |
| DNS en panne avec `reason="record"` bien que `expect` correspond | `expect_mode` est `exact` et la réponse porte une valeur supplémentaire, ou `forbid` a correspondu. |
| SMTP en panne avec `reason="protocol"` sur `security = starttls` | Le relais n'annonce pas STARTTLS sur ce port. Utilisez `tls` pour 465, `none` pour un relais local simple. |
| Une vérification de base de données en panne avec `reason="auth"` | Le compte existe mais ne peut peut-être pas se connecter depuis la machine DumbMonit : vérifiez `pg_hba.conf`, ou la partie hôte de l'utilisateur MySQL. |
| Une vérification de base de données en panne avec `reason="query"` | Connexion et login ont fonctionné. La base de données tourne et refuse de travailler : lecture seule, table manquante, récupération. |
| MQTT en panne avec `reason="payload"` sur un topic qui a clairement du trafic | Seuls les messages *retenus* atteignent un client qui vient de s'abonner. Publiez ce topic avec le drapeau retain, ou décochez « Attendre un message retenu ». |
| WebSocket en panne avec `reason="protocol"` après un `101` | Quelque chose sur le chemin répond à la mise à niveau sans parler WebSocket — généralement une règle de reverse proxy qui avale l'en-tête `Upgrade`. |

# Services : HTTP, TCP, DNS, ping, TLS, NTP, SMTP, SQL, MQTT, WebSocket

Onze moniteurs surveillent des **services** plutôt que du matériel, à la
façon d'Uptime Kuma : une page web, un port, un nom DNS, un hôte, un
certificat, un serveur de temps, un relais mail, une base de données, un
courtier MQTT, un point de terminaison WebSocket. Chaque service obtient
une barre d'historique, un temps de réponse et un pourcentage de
disponibilité sur sa page d'appareil.

Les cinq derniers vont plus loin que l'ouverture d'une connexion : ils
démarrent une vraie session dans le propre protocole du service. C'est la
différence entre « le port 5432 est ouvert » et « la base de données
accepte toujours mon compte et répond à une requête ». NTP se trouve
entre les deux : une seule requête et réponse, dans le propre protocole
de NTP, assez pour lire l'horloge du serveur sans session.

## Venir d'Uptime Kuma

**Surveillances > ⋯ > Importer depuis Uptime Kuma**, aussi sur
**Surveillances > Ajouter une surveillance** (administrateurs), lit le fichier de sauvegarde d'Uptime Kuma 1.x
(**Réglages > Sauvegarde > Exporter**, un fichier `.json` de 4 Mo au plus).
Uptime Kuma 2.x n'a plus d'export de sauvegarde : il n'y a donc rien à
importer depuis cette version. Rien n'est créé avant que vous ayez vu l'aperçu :
ce qui sera créé, et ce qui est ignoré avec la raison.

| Uptime Kuma | DumbMonit |
|---|---|
| HTTP(s), HTTP(s) - Mot-clé, HTTP(s) - Requête JSON | [HTTP](#http) : codes acceptés, méthode, mot-clé (et son inverse), chemin JSON et valeur attendue, redirections, délai, erreurs TLS ignorées |
| Ping | [Ping](#ping) |
| Port TCP | [TCP](#tcp), sous la forme `hôte:port` |
| DNS | [DNS](#dns) : type d'enregistrement et résolveur |
| Push | [Battement de cœur](push.md), période attendue = l'intervalle du moniteur. Il reçoit une **nouvelle** URL : mettez à jour la tâche qui l'appelle |
| Groupe | Un dossier (les groupes imbriqués donnent `Groupe/Sous-groupe`) |
| Étiquettes | Étiquettes de l'appareil : `nom=valeur`, ou `nom=true` pour une étiquette sans valeur |
| En pause | Créé désactivé |

Le nom et l'intervalle sont conservés (10 s au minimum). Tout le reste est
ignoré et listé : Docker et les autres types sans équivalent, et les moniteurs
en mode inversé. Un appareil déjà suivi (même type, même adresse) est ignoré
aussi : on peut donc importer deux fois le même fichier sans risque.

**Non importés :** les notifications (configurez vos canaux dans
[Notifications](../notifications.md) ; l'aperçu indique combien le fichier en
contenait), ainsi que les identifiants, en-têtes personnalisés et corps de
requête, qui ne sont jamais lus par DumbMonit. Si un site surveillé demande une
connexion ou un en-tête, ajoutez-le ensuite sur l'appareil.

## Comment fonctionne l'état d'un service

Un service peut répondre et être en panne quand même : un `500`, un mot-
clé manquant, un certificat expiré. Ces moniteurs écrivent donc un point
à chaque vérification, `probe_success` étant 1 ou 0, plutôt que de
devenir silencieux comme un appareil matériel. C'est ce qui rend un
pourcentage de disponibilité possible :
`avg_over_time(dumbmonit_probe_success[30d])`.

| Situation | Ce qui se passe |
|---|---|
| Option ou adresse invalide, sockets ICMP non autorisées (voir Ping) | Erreur de configuration : rien n'est écrit, la page de l'appareil montre l'erreur, aucune alerte « en panne ». |
| Connexion refusée, délai dépassé, TLS refusé, `500`, mot-clé absent, perte totale de paquets, mot de passe refusé, requête échouée | `probe_success = 0` : la panne est enregistrée, datée et comptée. |
| Tout va bien | `probe_success = 1`. |

Chaque moniteur a son propre délai (`timeout_seconds`, 5 s par défaut,
60 s au plus), plus court que le `DUMBMONIT_PROBE_TIMEOUT_SECS` du
serveur : interrompu par le planificateur, il ne pourrait pas écrire son
zéro.

Règles intégrées qui s'appliquent aux onze : Service en panne (3
minutes), Service instable (plus de six changements d'état en trente
minutes), Service lent (plus de 3 s pendant dix minutes). Pour chaque
vérification qui lit un certificat — `http`, `tls`, `smtp`, `mqtt`,
`websocket` : Certificat arrivant à expiration (14 jours) et Certificat
expiré.

Un mot de passe refusé n'est jamais une panne du service : il est
enregistré comme `reason="auth"`, l'alerte dit donc quoi corriger.

### Métriques

Toutes des jauges, toutes préfixées `dumbmonit_`, toutes étiquetées
`probe="http|tcp|dns|ping|tls|smtp|postgres|mysql|mqtt|websocket"` en
plus de `target`, `host` et `tag_*`.

| Métrique | Moniteurs | Signification |
|---|---|---|
| `probe_success` | tous | 1 si le service répond correctement, 0 sinon. |
| `probe_duration_seconds` | tous | Durée totale de la vérification. |
| `probe_failure_info` | tous | Présence (1) avec un label `reason` : `dns`, `connect`, `timeout`, `tls`, `cert_expired`, `status`, `keyword`, `json`, `body`, `packet_loss`, `record`, `auth`, `protocol`, `query`, `payload`. |
| `probe_http_status_code` | http, websocket | Code de statut obtenu. |
| `probe_http_first_byte_seconds` | http | Temps jusqu'aux en-têtes de réponse. |
| `probe_http_content_bytes` | http | Taille du corps. |
| `probe_http_security_header{header}` | http | 0/1 par étiquette `header` (https seulement) : HSTS, nosniff, CSP, X-Frame-Options, frame-ancestors. |
| `probe_connect_seconds` | tous sauf dns et ping | Établissement de connexion (connexion incluse pour les bases de données). |
| `probe_tls_handshake_seconds` | tls, http, smtp, mqtt, websocket | Négociation TLS seule. |
| `probe_tls_version_info` | tls, http, smtp, mqtt, websocket | Présence ; label `version`. |
| `probe_ssl_cert_expiry_days` | tls, http, smtp, mqtt, websocket | Jours avant expiration, négatif si expiré. |
| `probe_ssl_cert_valid` | tls, http, smtp, mqtt, websocket | 1 si la chaîne mène à une autorité connue. |
| `probe_ssl_cert_issuer_info` | tls, http, smtp, mqtt, websocket | Présence ; label `issuer`. |
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
| `probe_ws_message_bytes` | websocket | Taille de la trame reçue. |

## HTTP {#http}

**Site web ou API web (HTTP)** — vérifie qu'une page ou une API répond,
avec le bon code de statut, le bon contenu, et un certificat valide.
Exemples : un site web, une page de santé d'application, une API REST,
une interface de service auto-hébergé.

### Configuration

1. Dans l'adresse, collez l'URL complète de la page à surveiller. Sans « http:// » ou « https:// », HTTPS est supposé.
2. Préférez une page légère qui ne demande pas de connexion, par exemple la page de santé de l'application (« /health », « /status »), plutôt que la page d'accueil.
3. Si la page demande une authentification, remplissez l'identifiant : un nom d'utilisateur et mot de passe donnent l'authentification basique, un jeton est envoyé comme « Bearer ».
4. Par défaut, tout code de statut entre 200 et 299 convient. Pour aller plus loin, exigez un mot-clé dans la page ou une valeur spécifique dans une réponse JSON.
5. En HTTPS, le certificat est lu automatiquement : vous serez averti avant son expiration.

!!! warning
    N'écrivez jamais un mot de passe ou un jeton dans les options : ils sont copiés en texte clair sur chaque mesure. Utilisez le champ identifiant, qui est chiffré.

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

## TCP {#tcp}

**Port réseau (TCP)** — vérifie qu'un port accepte les connexions : SSH,
base de données, partage de fichiers, serveur de jeu, imprimante réseau.

### Configuration

1. Dans l'adresse, écrivez l'hôte suivi du port, séparés par deux-points : « nas.home.lan:22 » ou « 192.168.1.5:445 ».
2. Pour une adresse IPv6, mettez-la entre crochets : « [fd00::1]:445 ».
3. Vous pouvez aussi laisser l'adresse sans port et le saisir dans l'option « Port » : l'un des deux est requis.
4. Seule l'ouverture de connexion est testée : aucune donnée n'est envoyée au service, ça n'a donc aucun effet sur lui.

!!! warning
    Un port ouvert ne prouve pas que l'application derrière fonctionne. Pour un service web, préférez le type « Site web ou API web », qui lit la réponse.

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

### Configuration

1. Dans l'adresse, écrivez le nom à résoudre, sans « http:// » : « www.example.com ».
2. Par défaut, la vérification demande au résolveur système une adresse IPv4 (enregistrement A). Choisissez un autre type d'enregistrement dans les options si nécessaire.
3. Pour surveiller votre propre serveur DNS, saisissez son adresse IP dans « Serveur DNS à interroger » : la vérification échouera s'il arrête de répondre.
4. Pour détecter un détournement ou une mauvaise configuration, listez dans « Valeurs attendues » les adresses que la réponse doit contenir.

!!! warning
    Le serveur DNS est donné par son adresse IP, pas par un nom : il faudrait un résolveur pour résoudre le résolveur.

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

### Configuration

1. Dans l'adresse, écrivez le nom d'hôte ou l'adresse IP : « 192.168.1.1 » ou « router.home.lan ».
2. À chaque sonde, la vérification envoie quatre échos et mesure le temps de réponse, avec la part de paquets perdus.
3. Par défaut, seule une perte totale compte comme en panne ; une perte partielle reste visible dans les graphiques. Baissez « Perte tolérée » pour être averti plus tôt.
4. Si DumbMonit tourne dans Docker, gardez les lignes `sysctls:` du docker-compose.yml fourni sous le service dumbmonit : elles autorisent l'utilisateur non privilégié du conteneur à ouvrir des sockets d'écho ICMP. Aucune capacité n'est nécessaire.

!!! warning
    Sans ce sysctl, la vérification ne peut pas ouvrir de socket ICMP : elle le rapporte comme une erreur de configuration, pas comme une panne d'hôte. Certains appareils ignorent aussi les pings volontairement : vérifiez ça avant de tirer des conclusions.

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

### Configuration

1. Dans l'adresse, écrivez l'hôte et, s'il n'est pas 443, le port du service chiffré : « mail.example.com:993 », « ldap.home.lan:636 ».
2. La vérification ouvre une connexion chiffrée, lit le certificat présenté et ferme tout de suite : aucune donnée n'est échangée avec l'application.
3. Elle enregistre le nombre de jours avant expiration, l'émetteur et la version TLS. Les règles d'alerte fournies avertissent à quatorze jours, puis à l'expiration.
4. Si le service est atteint par son adresse IP mais que le certificat porte un nom, saisissez ce nom dans « Nom de serveur (SNI) ».
5. Pour un certificat émis par votre propre autorité, cochez « Accepter un certificat non vérifiable » : la date d'expiration est quand même surveillée.

!!! warning
    Pour un site web, le type « Site web ou API web » lit déjà le certificat : ce type est destiné aux services qui ne parlent pas HTTP, ou dont vous ne voulez pas solliciter l'application.

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

**Relais mail (SMTP)** — ouvre une vraie session sur un serveur mail :
salutation, EHLO, STARTTLS, et la connexion si vous en donnez une.
Exemples : le relais SMTP de votre fournisseur, un Postfix ou msmtp
local, Proxmox Mail Gateway, le port de soumission d'un serveur mail.

### Configuration

1. Dans l'adresse, écrivez le serveur mail, et le port s'il n'est pas l'habituel : « smtp.example.com » ou « smtp.example.com:2525 ».
2. Choisissez le chiffrement que votre relais utilise : STARTTLS pour le port de soumission 587, TLS pour le port 465, Aucun pour un relais sur votre propre réseau écoutant sur le port 25.
3. Pour vérifier que le relais accepte toujours votre compte, remplissez l'identifiant : la vérification lance alors AUTH et rapporte un mot de passe refusé comme tel, pas comme une panne.
4. Rien n'est jamais envoyé : la vérification s'arrête après la salutation, les extensions et la connexion optionnelle, puis raccroche. Aucun message n'entre dans la file.
5. Pour être averti quand une extension disparaît, nommez-la dans « Extension attendue » : un relais qui arrête d'annoncer STARTTLS est un relais qui enverrait votre mail en texte clair.

!!! warning
    Une session acceptée ne prouve pas que le mail part. Un relais qui accepte tout et le met en file pour toujours répond parfaitement ici : surveillez les files elles-mêmes pour ça.

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

**Base de données PostgreSQL** — se connecte, s'authentifie et lance une
requête : le signal d'une base de données qui tourne mais ne répond
plus. Exemples : la base derrière Nextcloud ou Immich, un recorder Home
Assistant, une base d'application, une instance TimescaleDB.

### Configuration

1. Sur le serveur, créez un compte pour la surveillance et ne lui donnez rien de plus que le droit de se connecter : CREATE ROLE dumbmonit LOGIN PASSWORD 'a-long-password';
2. Assurez-vous que ce compte peut atteindre le serveur depuis la machine DumbMonit, ce qui demande généralement une ligne de plus dans pg_hba.conf suivie d'un rechargement de configuration.
3. Dans l'adresse, écrivez le serveur, et le port s'il n'est pas 5432 : « db.home.lan » ou « db.home.lan:5433 ».
4. La requête par défaut est SELECT 1, qui ne lit aucune table : rien d'autre n'a besoin d'être accordé. Temps de connexion et temps de requête sont enregistrés séparément.
5. Pour surveiller quelque chose de vôtre, remplacez la requête par une qui renvoie un seul nombre, et ça devient un graphique : une longueur de file, un nombre de lignes, un retard de réplication.

!!! warning
    La vérification ouvre une vraie connexion à chaque sonde. Sur une instance déjà proche de sa limite de connexions, donnez-lui un intervalle plus long.

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

**Base de données MySQL ou MariaDB** — se connecte, s'authentifie et
lance une requête : le signal d'une base de données qui tourne mais ne
répond plus. Exemples : la base derrière un WordPress, un Nextcloud ou un
Kimai, une base d'application, un MariaDB en Docker.

### Configuration

1. Sur le serveur, créez un compte pour la surveillance sans aucun privilège : CREATE USER 'dumbmonit'@'%' IDENTIFIED BY 'a-long-password';
2. Vérifiez que le compte peut se connecter depuis la machine DumbMonit : un motif d'hôte de localhost n'accepterait que les connexions faites depuis le serveur lui-même.
3. Dans l'adresse, écrivez le serveur, et le port s'il n'est pas 3306 : « db.home.lan » ou « db.home.lan:3307 ».
4. La requête par défaut est SELECT 1, qui ne lit aucune table : rien d'autre n'a besoin d'être accordé. Temps de connexion et temps de requête sont enregistrés séparément.
5. Pour surveiller quelque chose de vôtre, remplacez la requête par une qui renvoie un seul nombre, et ça devient un graphique : une longueur de file, un nombre de lignes, un retard de réplication.

!!! warning
    La vérification ouvre une vraie connexion à chaque sonde. Sur une instance déjà proche de sa limite de connexions, donnez-lui un intervalle plus long.

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

**Courtier MQTT** — se connecte au courtier, s'abonne à un topic, et
peut attendre un message retenu. Exemples : Mosquitto, le courtier
derrière Home Assistant, Zigbee2MQTT, les capteurs ESPHome.

### Configuration

1. Dans l'adresse, écrivez le courtier, et le port s'il n'est pas l'habituel : « broker.home.lan » ou « broker.home.lan:1884 ».
2. Cochez « Connexion chiffrée » pour un courtier écoutant sur 8883, et remplissez l'identifiant s'il demande un compte.
3. Se connecter vaut déjà la peine d'être surveillé : un courtier qui refuse votre compte, ou qui a arrêté de répondre, est la raison pour laquelle toute la maison est devenue silencieuse.
4. Pour aller plus loin, nommez un topic : la vérification s'y abonne alors, et un courtier qui refuse l'abonnement vous dit que le compte a perdu ses droits d'accès.
5. Cochez « Attendre un message retenu » seulement pour un topic qui en porte un, typiquement un topic de disponibilité. Un topic publié de temps en temps paraît silencieux à un client qui vient juste de se connecter.

!!! warning
    La vérification ne publie jamais rien. Elle ne peut non plus dire l'âge d'un message retenu : MQTT ne les date pas, une valeur gelée depuis trois semaines compte donc toujours comme présente.

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
| `client_id` | Identifiant client | *(dérivé)* | Nom annoncé au courtier. Le défaut dérive de l'appareil, deux vérifications ne se déconnectent donc jamais l'une l'autre. |
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

**Point de terminaison WebSocket** — lance la poignée de main de mise à
niveau, et peut envoyer une trame et en attendre une : une panne
différente d'une simple page HTTP. Exemples : l'API Home Assistant, un
tableau de bord en direct, un flux de logs, un point de terminaison
derrière un reverse proxy.

### Configuration

1. Dans l'adresse, collez le point de terminaison complet : « wss://home.example.com/api/websocket ». Sans schéma, wss est supposé.
2. La vérification lance toute la poignée de main d'ouverture et vérifie la réponse que le serveur calcule depuis la clé qui lui a été envoyée : un reverse proxy qui a perdu l'en-tête Upgrade est attrapé ici, là où une vérification HTTP verrait encore un 200 sain.
3. Rien d'autre n'est nécessaire pour la plupart des points de terminaison. Un statut autre que 101 est rapporté avec son code, un 401 d'un jeton expiré n'est donc pas confondu avec une panne.
4. Pour aller plus loin, remplissez « Contenu attendu » avec un fragment de la première trame que le serveur envoie de lui-même, par exemple la salutation de l'API Home Assistant.
5. Si le point de terminaison ne dit rien jusqu'à ce qu'on lui parle, mettez un message dans « Trame à envoyer » et ce que sa réponse doit contenir dans « Contenu attendu ».

!!! warning
    La vérification ouvre et raccroche en moins d'une seconde. Une connexion qu'un pare-feu ou un délai de proxy coupe après trente secondes paraît parfaitement saine ici.

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

### Configuration

1. Dans l'adresse, écrivez le nom ou l'adresse IP du serveur : « pool.ntp.org » ou « 192.168.1.1 ». Le port UDP 123 est utilisé sauf indication contraire de l'option « Port ».
2. La vérification envoie une seule requête SNTP et lit la réponse : aucun état n'est gardé entre deux sondes, et rien n'est écrit sur le serveur.
3. Elle enregistre le décalage entre l'horloge du serveur et l'hôte DumbMonit, le délai d'aller-retour, le stratum et l'indicateur de saut.
4. Un serveur qui répond stratum 16, ou dont l'indicateur de saut lit « non synchronisé », n'a jamais réussi à se synchroniser : la vérification le rapporte en panne quel que soit le décalage.
5. Baissez « Décalage toléré » pour être averti plus tôt que 100 ms ; augmentez-le pour un serveur à plusieurs sauts d'une horloge de référence.

!!! warning
    Le décalage n'est aussi bon que la propre horloge de l'hôte DumbMonit : si cet hôte n'est pas lui-même synchronisé (NTP, la synchro de temps d'un hyperviseur), le décalage mesuré reflète sa propre dérive autant que celle du serveur.

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

## Adresses que les vérifications refusent

Les vérifications qui se connectent à une adresse que vous leur donnez —
HTTP, TCP, TLS, NTP, SMTP, PostgreSQL, MySQL, MQTT, WebSocket —
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

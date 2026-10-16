# MDaemon Email Server

Les services mail d'un serveur MDaemon, vérifiés depuis l'extérieur : SMTP,
IMAP, POP3, webmail et Remote Administration acceptent les connexions et
saluent comme il faut. Avec un compte, DumbMonit lit aussi la version
exacte via l'API XML. Avec l'[agent](agent.md) installé sur le serveur
MDaemon, la même page montre aussi les files de mail, les sessions et les
totaux de messages, que MDaemon ne publie que comme compteurs de
performance Windows.

!!! warning "Validé contre la documentation seulement"

    Cette intégration a été construite à partir de la documentation
    publiée par MDaemon Technologies et du format de réponse de l'API XML
    montré là, pas contre un vrai serveur MDaemon. Chaque appel est une
    lecture, et les vérifications de port mail suivent les standards de
    protocole, mais personne n'a encore fait tourner cela contre un
    MDaemon en fonctionnement. Pareil pour les compteurs de file lus par
    l'agent : leurs noms viennent d'une liste publiée des compteurs de
    performance de MDaemon, pas d'un serveur en fonctionnement. Dites-nous
    ce qui casse.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_mdaemon_`.

| Métrique | Labels | Signification |
|---|---|---|
| `service_up` | `service`, `port` | 1 si le port a accepté la connexion et, pour SMTP, POP3 et IMAP, a salué avec `220`, `+OK` ou `* OK`. 0 s'il n'a pas répondu, ou a répondu pour refuser le service (`421`, `554`, `-ERR`, `* BYE`). |
| `service_response_seconds` | `service`, `port` | Temps jusqu'à la salutation, ou jusqu'à la connexion pour un port qui n'en a pas. |
| `api_up` | | Avec un compte seulement : 1 si l'API XML a répondu, 0 si elle n'a pas répondu. |
| `api_response_seconds` | | Temps que l'API XML a pris pour répondre. |
| `api_operation_ok` | | 1 si le compte était autorisé à lancer l'appel, 0 si MDaemon a répondu avec un statut d'erreur. |
| `info` (valeur 1) | `version`, `build`, `source` | La version : depuis l'API XML (`source="xml_api"`, `build` est le build du service), ou sinon depuis la salutation SMTP quand le serveur l'y annonce (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Temps que toute la vérification a pris. |

**Une panne partielle reste une vérification réussie.** Un service arrêté
donne `service_up 0` pour ce service et les autres sont toujours mesurés.
C'est seulement quand aucun service et aucune API ne répondent du tout que
la surveillance est *inaccessible*.

**Un compte refusé est une erreur de configuration, pas une panne.** Un
mauvais mot de passe, une adresse que l'API XML n'autorise pas, ou un
mauvais port sont affichés sur la page de la surveillance avec la raison et ne
sont pas notifiés.

**Les files viennent de l'agent, pas de cette surveillance.** MDaemon publie la
taille de ses files (inbound, local, remote, retry, bad, holding,
quarantine), ses compteurs de session, ses statistiques de message et son
uptime comme compteurs de performance Windows, dans l'objet nommé MDaemon,
et dans son propre Queue and Statistics Manager. Aucune documentation
publiée de MDaemon ne décrit un appel d'API XML qui les renvoie, cette surveillance n'en devine donc pas un : l'[agent](agent.md), installé sur le
serveur MDaemon, lit les compteurs de performance à la place. Voir
[Files de mail via l'agent](#mail-queues-through-the-agent). La licence et
l'espace disque du stockage de mail ne sont exposés par aucune interface
documentée ni compteur de performance, et ne sont pas lus. L'agent
surveille aussi le disque qui contient le stockage de mail ; listez le
service MDaemon dans son réglage `services` pour surveiller le propre
service Windows.

Les [règles intégrées](../alerting/rules.md#mdaemon-and-securitygateway)
qui s'appliquent : service mail MDaemon arrêté, API XML MDaemon ne répond
pas, plus Surveillance inaccessible ; avec l'agent, file de mail MDaemon qui
grandit, file Bad MDaemon non vide et file Retry MDaemon élevée.

## La page de la surveillance

Une surveillance MDaemon montre un panneau **Services mail** au-dessus des
graphiques, lu depuis la dernière mesure stockée : ouvrir la page ne se
connecte jamais au serveur. Il liste chaque service surveillé avec son
port, un mot et une couleur (Répond, Arrêté), et son temps de réponse ;
puis la version et où elle a été lue. Les services arrêtés viennent
d'abord.

Quand l'agent sur le même serveur rapporte les compteurs de MDaemon, un
panneau **Files de mail** suit : le nombre de messages dans chaque file
(une file gelée, une file Bad non vide et plus de cinquante messages en
Retry sont signalés en mots), les sessions actives, les messages et les
verdicts spam, virus et DNSBL des dernières 24 heures, l'uptime de
MDaemon, et les serveurs internes que MDaemon rapporte comme inactifs. Le
même panneau apparaît sur la propre page de l'agent.

## Files de mail via l'agent {#mail-queues-through-the-agent}

1. Installez l'[agent](agent.md#install) sur le serveur Windows qui fait
   tourner MDaemon. Rien d'autre n'est nécessaire : l'agent remarque le
   service Windows nommé `MDaemon` et lit les compteurs de l'objet de
   performance MDaemon à chaque période d'échantillonnage. Pour le forcer
   activé ou désactivé, réglez `mdaemon: true` ou `mdaemon: false` dans
   `agent.yaml`.
2. Les files apparaissent sur la page de l'agent. Pour aussi les montrer
   sur cette surveillance MDaemon, DumbMonit cherche l'agent sur la même
   machine : l'agent réglé comme parent de cette surveillance, ou l'agent relais
   qui le sonde, ou l'agent dont le nom d'hôte correspond à l'adresse de
   cette surveillance (`mail` pour `mail.example.com`), ou, quand il y a une surveillance MDaemon et un agent rapportant les compteurs MDaemon, cet
   agent. Si aucun ne correspond, modifiez cette surveillance et choisissez
   l'agent comme sa **Surveillance parente** : l'agent et MDaemon partagent la
   machine, donc quand l'agent devient silencieux les alertes de cette surveillance sont aussi retenues.

Les séries appartiennent à l'agent, avec son label `target`. Toutes sont
préfixées `dumbmonit_mdaemon_` :

| Métrique | Labels | Compteurs de performance lus |
|---|---|---|
| `queue_messages` | `queue` : `inbound`, `local`, `remote`, `retry`, `bad`, `holding`, `lan`, `quarantine`, `raw` | `Inbound queue messages`, `Local queue messages`, `Remote queue messages`, `Retry queue messages`, `Bad queue messages`, `Holding queue messages`, `LAN queue messages`, `Quarantine queue messages`, `RAW queue messages` |
| `queue_frozen` (1 gelée) | `queue` : `inbound`, `local`, `remote` | `Inbound queue frozen`, `Local queue frozen`, `Remote queue frozen` |
| `sessions_active` | `protocol` : `smtp_in`, `smtp_out`, `pop3_in`, `pop3_out`, `imap`, `webmail` | `Active SMTP (in) sessions`, `Active SMTP (out) sessions`, `Active POP3 (in) sessions`, `Active POP3 (out) sessions`, `Active IMAP sessions`, `Active Webmail sessions` |
| `sessions_total` (compteur) | `protocol` : `smtp_in`, `smtp_out`, `pop3`, `imap` | `SMTP sessions (in) total`, `SMTP sessions (out) total`, `POP3 sessions total`, `IMAP sessions total` |
| `messages_total` (compteur) | `protocol` : `smtp_in`, `smtp_out`, `domainpop_in` | `SMTP messages (in) total`, `SMTP messages (out) total`, `DomainPOP messages (in) total` |
| `filtered_messages_total` (compteur) | `filter` : `spam`, `virus`, `dnsbl` ; `verdict` : `accepted`, `refused` | `spam accepted total`, `spam refused total`, `Viruses accepted total`, `Viruses refused total`, `DNSBL accepted total`, `DNSBL refused total` |
| `server_active` (1 actif) | `server` : `smtp`, `pop3`, `imap`, `webmail`, `webadmin`, `activesync`, `antispam`, `antivirus`, `minger`, `multipop`, `domainpop` | `SMTP server state`, `POP3 server state`, `IMAP server state`, `Web Mail server state`, `Web Admin server state`, `ActiveSync server state`, `AntiSpam server state`, `AntiVirus server state`, `Minger server state`, `MultiPOP server state`, `DomainPOP server state` |
| `running` (1 en marche) | | `MDaemon running state` |
| `uptime_seconds` | | `MDaemon up time` |

Les totaux comptent depuis le dernier démarrage de MDaemon : appliquez
`increase()` ou `rate()` dans les requêtes, comme la page le fait pour ses
chiffres sur 24 heures. Les compteurs par seconde
(`SMTP messages (in)/sec`…) ne sont pas lus, puisque les totaux donnent les
mêmes débits. Un compteur que votre version de MDaemon n'a pas, ou nomme
différemment, est rapporté une fois dans le journal de l'agent
(`performance counter not available`) et les autres sont quand même lus ;
pour en lire un que le préréglage n'a pas, listez-le sous
`perf_counters` dans `agent.yaml` (voir
[Compteurs de performance Windows](agent.md#windows-performance-counters)).

## Surveiller les services mail, et optionnellement l'API XML

1. Rien à installer sur le serveur. Sans identifiant, DumbMonit se connecte aux ports mail depuis l'extérieur et lit chaque salutation : SMTP (25), IMAP (143) et Webmail (3000) par défaut. Ajoutez POP3, MSA, les ports TLS ou Remote Administration dans l'option Services, comme une liste telle que celle-ci.

    ```
    smtp,msa,imap,pop3,imaps,webmail
    ```

2. Pour aussi lire la version via l'API XML, créez un compte dédié dans MDaemon, nommé par exemple comme suit, avec un mot de passe long utilisé nulle part ailleurs. Donnez-lui le niveau le plus bas que votre MDaemon accepte pour l'API XML, et rien de plus : DumbMonit n'appelle que GetVersionInfo, qui ne lit aucune boîte mail et ne change rien.

    ```
    dumbmonit@example.com
    ```

3. Depuis MDaemon 24, l'API XML ne répond qu'aux adresses qu'elle autorise. Ouvrez Setup → XML API Service → Address Restrictions (Setup → XML API Management sur MDaemon 26) et autorisez l'adresse depuis laquelle DumbMonit se connecte.

4. Vérifiez depuis l'hôte DumbMonit que le compte atteint l'API. La commande demande le mot de passe et affiche une réponse qui commence par `<MDaemon><API productversion=…>`.

    ```
    curl -k -u dumbmonit@example.com -H 'Content-Type: text/xml' --data '<MDaemon><API><Request version="20.0.0" echo="0" verbose="0"><Operation>GetVersionInfo</Operation><Parameters/></Request></API></MDaemon>' https://mail.example.com:444/MdMgmtWS/
    ```

5. Dans DumbMonit, saisissez l'adresse du serveur, par exemple « mail.example.com », puis l'adresse email complète du compte comme nom d'utilisateur, et son mot de passe. L'API XML est atteinte sur le port HTTPS de Remote Administration, 444 par défaut.

!!! warning
    MDaemon n'a pas de rôle en lecture seule pour l'API XML : ne réutilisez jamais votre propre compte, et gardez ce mot de passe hors de tout autre outil. Dynamic Screening peut bloquer une adresse après des connexions échouées répétées, donc un mauvais mot de passe ici peut faire bloquer l'hôte DumbMonit : testez d'abord avec la commande ci-dessus. Remote Administration utilise souvent un certificat autosigné : si la connexion est refusée pour cette raison, cochez « Accepter un certificat non vérifiable » dans les options. Les tailles de file mail ne sont pas lues par cette surveillance : MDaemon ne les publie que comme compteurs de performance Windows, que l'agent DumbMonit lit quand il est installé sur le serveur mail.

L'API XML est servie par le serveur web de Remote Administration : port
1000 en HTTP et 444 en HTTPS par défaut, sur `/MdMgmtWS/`, ou par IIS quand
Remote Administration y tourne. Sa référence complète est fournie avec le
serveur : ouvrez `https://mail.example.com:444/MdMgmtWS` dans un
navigateur, ou cherchez dans `\MDaemon\Docs\API\XML API\`.

## Identifiants

| Identifiant | Champs |
|---|---|
| Services seulement | Rien : les ports mail sont vérifiés sans se connecter. |
| Compte API XML | L'adresse email complète du compte, et son mot de passe, envoyés en authentification HTTP basique. |

Adresse : le nom d'hôte ou l'IP du serveur, par exemple
`mail.example.com`. Un port écrit dans l'adresse
(`mail.example.com:8444`) est pris comme le port de l'API XML.

## Options

| Option | Défaut | Effet |
|---|---|---|
| `services` | `smtp,imap,webmail` | Quoi vérifier, séparé par des virgules : `smtp` (25), `msa` (587), `smtps` (465), `pop3` (110), `pop3s` (995), `imap` (143), `imaps` (993), `webmail` (3000), `remote_admin` (1000), `remote_admin_https` (444), `xmpp` (5222). `nom:port` change un port ; tout autre nom avec son port ajoute un service propre, vérifié par connexion seulement. `none` ne vérifie aucun port. |
| `request_timeout_seconds` | `10` | Temps accordé à chaque connexion, salutation et appel d'API, de 1 à 60. |
| `api_port` | `444` | Le port Remote Administration qui sert `/MdMgmtWS/`. Utilisé seulement avec un compte. |
| `api_tls` | `true` | HTTPS vers l'API XML. Décochez seulement pour Remote Administration en HTTP simple (port 1000 par défaut) : le mot de passe traverse alors le réseau en clair. |
| `insecure_tls` | `false` | Accepter un certificat autosigné sur l'API XML. |

Les ports TLS (`smtps`, `pop3s`, `imaps`, `remote_admin_https`) sont
vérifiés par connexion seulement. Pour surveiller l'expiration de leurs
certificats, ajoutez une surveillance
[Certificat TLS](services.md#tls) pour chacun.

# MDaemon Email Server

MDaemon se surveille avec **l’agent DumbMonit installé sur le serveur Windows
qui le fait tourner**. Les files de mail, les sessions, les services Windows
et l’espace disque du stockage de mail ne se lisent que sur le serveur
lui-même : MDaemon ne les publie nulle part ailleurs, et aucune interface
distante ne les renvoie. L’agent les lit localement et les envoie à
DumbMonit ; la surveillance MDaemon relie ensuite l’agent aux ports mail que
DumbMonit vérifie depuis l’extérieur (SMTP, IMAP, POP3, messagerie web,
Remote Administration).

!!! info "Agent requis sur le serveur de messagerie"

    Sans l’[agent](agent.md), DumbMonit ne voit que les ports mail et, si vous
    donnez un compte, la version. La page de la surveillance le dit dans un
    bandeau : *Agent non installé ou non lié*, ou *Agent injoignable*. Ce
    bandeau est un avertissement sur la page, pas une panne : aucune
    notification n’est envoyée pour lui.

| | Avec l’agent sur le serveur | Sans lui |
|---|---|---|
| Ports mail (SMTP, IMAP, POP3, messagerie web, Remote Administration) | Vérifiés | Vérifiés |
| Services Windows de MDaemon (état, mode de démarrage) | Lus | Non surveillés |
| Files (inbound, local, remote, retry, bad, holding…) et leur plus ancien message | Lues | Non surveillées |
| Sessions actives, totaux de messages, verdicts spam et virus | Lus | Non surveillés |
| Version | Lue | Seulement avec un compte de l’API XML, ou depuis la salutation SMTP |
| Espace disque du stockage de mail | Lu | Non surveillé |

## Ports réseau à ouvrir

De l’agent vers le serveur DumbMonit :

| Port | Protocole | Usage |
|---|---|---|
| 8080 | TCP | Mesures de l’agent (port par défaut, 443 derrière un proxy TLS) |

Du serveur DumbMonit (ou de son agent relais) vers l’équipement :

| Port | Protocole | Usage |
|---|---|---|
| 25 | TCP | Vérification SMTP (par défaut) |
| 143 | TCP | Vérification IMAP (par défaut) |
| 3000 | TCP | Vérification de la messagerie web (par défaut) |
| 444 | TCP | Seulement avec un compte de l’API XML (HTTPS, par défaut) |
| 1000 | TCP | Seulement avec l’API XML en HTTP |
| 587 | TCP | Seulement si msa figure dans l’option Services |
| 465 | TCP | Seulement si smtps figure dans l’option Services |
| 110 | TCP | Seulement si pop3 figure dans l’option Services |
| 995 | TCP | Seulement si pop3s figure dans l’option Services |
| 993 | TCP | Seulement si imaps figure dans l’option Services |
| 5222 | TCP | Seulement si xmpp figure dans l’option Services |

L’agent appelle le serveur, jamais l’inverse : aucun port n’a besoin d’être
ouvert sur le serveur de messagerie pour lui.

!!! warning "Validé contre la documentation seulement"

    Cette intégration a été construite à partir de la documentation publiée
    par MDaemon Technologies, pas contre un vrai serveur MDaemon. Chaque appel
    est une lecture, et les vérifications de port mail suivent les standards
    de protocole, mais personne n’a encore fait tourner cela contre un
    MDaemon en fonctionnement. C’est vrai en particulier pour les noms des
    services Windows de MDaemon, les dossiers de ses files (`Queues\Inbound`,
    `Retry`, `Bad`…) et les noms de ses compteurs de performance, qui
    viennent de la documentation publiée et non d’un serveur en
    fonctionnement. Dites-nous ce qui casse.

## Installer

1. Installez l’[agent](agent.md#install) sur le serveur Windows qui fait
   tourner MDaemon : dans DumbMonit, ajoutez une surveillance **Serveur avec
   agent** et lancez sa commande PowerShell dans un PowerShell élevé. Depuis
   **Ajouter un équipement** → **MDaemon Email Server**, le bouton *Installer
   l’agent Windows* ouvre ce parcours avec la seule commande PowerShell, et le
   bouton *Ajouter la surveillance MDaemon* vous ramène ici.
2. Ajoutez la surveillance MDaemon et choisissez cet agent dans **Agent
   installé sur ce serveur**. Quand l’adresse saisie a le nom d’hôte d’un
   agent, DumbMonit le présélectionne. Sans ce choix, la page se rabat sur la
   recherche de l’agent par elle-même (l’agent réglé comme parent de la
   surveillance, l’agent relais qui la sonde, l’agent dont le nom d’hôte
   correspond à l’adresse), mais seul le choix explicite vous prévient, dans
   un bandeau, quand l’agent est absent ou silencieux.
3. Laissez l’identifiant sur *Services seulement*. Un compte de l’API XML est
   facultatif et n’ajoute que la version exacte.

L’agent détecte MDaemon sans aucun réglage : il cherche un service Windows
dont le nom ou le nom affiché contient `MDaemon`, prend le dossier
d’installation dans le chemin de son exécutable (`C:\MDaemon\App\MDaemon.exe`
donne `C:\MDaemon`), et lit les compteurs de performance du produit. Pour le
forcer activé ou désactivé, réglez `mdaemon: true` ou `mdaemon: false` dans
`agent.yaml` ; `false` désactive aussi la lecture des services Windows et des
dossiers de files décrite ci-dessous.

## Ce qu’il surveille

### Lu par l’agent {#mail-queues-through-the-agent}

Les services Windows, les dossiers de files, la version et l’espace disque
viennent de la propre lecture de l’agent. Ils sont préfixés
`dumbmonit_agent_mail_`, appartiennent au `target` de l’agent, et portent
`product="mdaemon"`.

| Métrique | Labels | Signification |
|---|---|---|
| `info` (valeur 1) | `product`, `version` | MDaemon a été trouvé sur le serveur, avec la version produit de son exécutable (absente si illisible). |
| `service_up` | `product`, `service`, `display`, `start` | 1 si le service Windows tourne. `service` est son nom, `display` son nom affiché, `start` la façon dont Windows le démarre : `auto`, `manual`, `disabled` ou `other`. Tout service dont le nom ou le nom affiché contient `MDaemon` est listé (le moteur, Remote Administration, XMPP, antivirus…, selon l’installation). |
| `queue_files` | `product`, `queue` : `inbound`, `local`, `remote`, `retry`, `bad`, `holding`, `lan`, `quarantine`, `raw` | Messages (fichiers `.msg`) dans le dossier de la file sous `Queues\` dans le dossier d’installation. Seulement les dossiers qui existent ; 20 000 fichiers au plus sont comptés par dossier. |
| `queue_oldest_seconds` | `product`, `queue` | Âge du plus ancien message d’un dossier de file non vide. |
| `data_free_bytes`, `data_total_bytes` | `product` | Espace libre et total du volume qui contient le dossier d’installation. |

Les tailles sont comptées depuis les dossiers pour qu’elles existent même là
où les compteurs de MDaemon manquent. Quand les compteurs ci-dessous
existent, la page montre leurs valeurs ; le comptage des dossiers complète les
autres files.

MDaemon publie aussi la taille de ses files (inbound, local, remote, retry,
bad, holding, quarantine), ses compteurs de session, ses statistiques de
message et son uptime comme compteurs de performance Windows, dans l’objet
nommé MDaemon. L’agent les lit ; aucune documentation publiée de MDaemon ne
décrit un appel d’API XML qui les renvoie. Les séries sont préfixées
`dumbmonit_mdaemon_` :

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
mêmes débits. Un compteur que votre version de MDaemon n’a pas, ou nomme
différemment, est rapporté une fois dans le journal de l’agent
(`performance counter not available`) et les autres sont quand même lus ;
pour en lire un que le préréglage n’a pas, listez-le sous
`perf_counters` dans `agent.yaml` (voir
[Compteurs de performance Windows](agent.md#windows-performance-counters)).

### Services réseau

Vérifiés par DumbMonit depuis l’extérieur, sans compte. Toutes les métriques
sont préfixées `dumbmonit_mdaemon_`.

| Métrique | Labels | Signification |
|---|---|---|
| `service_up` | `service`, `port` | 1 si le port a accepté la connexion et, pour SMTP, POP3 et IMAP, a salué avec `220`, `+OK` ou `* OK`. 0 s’il n’a pas répondu, ou a répondu pour refuser le service (`421`, `554`, `-ERR`, `* BYE`). |
| `service_response_seconds` | `service`, `port` | Temps jusqu’à la salutation, ou jusqu’à la connexion pour un port qui n’en a pas. |
| `scrape_duration_seconds` | | Temps que toute la vérification a pris. |

### API (facultative)

Avec un compte de l’API XML seulement.

| Métrique | Labels | Signification |
|---|---|---|
| `api_up` | | 1 si l’API XML a répondu, 0 si elle n’a pas répondu. |
| `api_response_seconds` | | Temps que l’API XML a pris pour répondre. |
| `api_operation_ok` | | 1 si le compte était autorisé à lancer l’appel, 0 si MDaemon a répondu avec un statut d’erreur. |
| `info` (valeur 1) | `version`, `build`, `source` | La version : depuis l’API XML (`source="xml_api"`, `build` est le build du service), ou sinon depuis la salutation SMTP quand le serveur l’y annonce (`source="smtp_banner"`). |

**Une panne partielle reste une vérification réussie.** Un service arrêté
donne `service_up 0` pour ce service et les autres sont toujours mesurés.
C’est seulement quand aucun service et aucune API ne répondent du tout que
la surveillance est *inaccessible*.

**Un compte refusé est une erreur de configuration, pas une panne.** Un
mauvais mot de passe, une adresse que l’API XML n’autorise pas, ou un
mauvais port sont affichés sur la page de la surveillance avec la raison et ne
sont pas notifiés.

La licence n’est exposée par aucune interface documentée ni compteur de
performance, et n’est pas lue.

Les [règles intégrées](../alerting/rules.md#mdaemon-and-securitygateway) qui
s’appliquent : service mail MDaemon arrêté et API XML MDaemon ne répond pas
(depuis la surveillance), plus Surveillance inaccessible ; et, sur la
surveillance de l’agent, Service Windows du serveur mail arrêté, file de mail
MDaemon qui grandit, File de mail bloquée, File MDaemon gelée, File Bad
MDaemon non vide et File Retry MDaemon élevée.

## La page de la surveillance

La page d’une surveillance MDaemon montre, de haut en bas :

1. Un **bandeau** quand l’agent est absent ou silencieux : *Agent non installé
   ou non lié* (avec un bouton pour l’installer et un pour le choisir),
   *Agent injoignable* (avec l’heure de son dernier rapport), ou *L’agent ne
   voit pas MDaemon sur sa machine*. Il prévient que les files et les
   services Windows ne sont pas surveillés ; ce n’est pas une alerte.
2. **Lu par l’agent** : les *Services Windows* (en marche ou arrêtés, et leur
   mode de démarrage), les *Files* (messages par file, l’âge du plus ancien,
   et un mot quand une file est gelée, bloquée, ou quand Bad n’est pas vide
   ou Retry dépasse cinquante), les *Sessions*, les messages et les verdicts
   spam, virus et DNSBL des dernières 24 heures, l’uptime et la version de
   MDaemon, l’espace disque du stockage de mail, et les serveurs internes que
   MDaemon rapporte comme inactifs. Le même panneau apparaît sur la propre
   page de l’agent.
3. **Services réseau** : chaque port surveillé, un mot et une couleur (Répond,
   Arrêté), le temps de réponse ; puis, sous **API**, l’état de l’API XML et
   la version qu’elle lit.

Tout est lu depuis la dernière mesure stockée : ouvrir la page ne se connecte
jamais au serveur.

## Installer l’agent, puis ajouter la surveillance

1. Sur le serveur Windows qui fait tourner MDaemon, ajoutez une surveillance Serveur avec agent dans DumbMonit et lancez sa commande PowerShell dans un PowerShell élevé. L’agent trouve MDaemon tout seul.

2. De retour ici, saisissez l’adresse du serveur et choisissez cet agent dans Agent installé sur ce serveur. DumbMonit présélectionne celui qui a le même nom d’hôte.

3. Laissez l’identifiant sur Services seulement : l’agent apporte déjà les files, les sessions, les services Windows et l’espace disque, et les ports mail sont vérifiés depuis DumbMonit.

4. Facultatif, seulement pour lire la version exacte : ouvrez Accounts → Account Manager → New account et créez le compte ci-dessous, avec un long mot de passe unique.

    ```
    dumbmonit@example.com
    ```

5. Autorisez l’adresse depuis laquelle DumbMonit se connecte dans Setup → XML API Service → Address Restrictions (Setup → XML API Management sur MDaemon 26), puis saisissez l’adresse e-mail complète du compte et son mot de passe.

6. Pour surveiller d’autres ports que SMTP, IMAP et la messagerie web, réglez l’option Services sur une liste comme celle-ci.

    ```
    smtp,msa,imap,pop3,imaps,webmail
    ```

### Droits requis

- **Administrateur local du serveur** : Seulement pour installer l’agent, qui tourne ensuite comme service Windows
- **Accès à l’API XML pour le compte** : Facultatif : seulement pour lire la version
- **Restriction d’adresse de l’API XML** : Facultatif : autoriser l’adresse depuis laquelle DumbMonit se connecte

Documentation de l’éditeur : <https://help.mdaemon.com/mdaemon/en/xml-api-service.html>

### Bon à savoir

Les files, les sessions et les services Windows sont lus par l’agent, avec les droits du service Windows qui le fait tourner (Local System par défaut), qui peut lire le dossier d’installation et les compteurs de performance. Rien n’est écrit sur le serveur.

Le compte de l’API XML ne sert qu’à lire la version : DumbMonit n’appelle que `GetVersionInfo`, qui ne lit aucune boîte et ne change rien. Depuis MDaemon 24, l’API XML ne répond qu’aux adresses qu’elle autorise. Elle est atteinte sur le port HTTPS de l’administration à distance, 444 par défaut. Pour vérifier depuis l’hôte DumbMonit que le compte atteint l’API, lancez la commande ci-dessous : elle demande le mot de passe et affiche une réponse qui commence par `<MDaemon><API productversion=…>`.

```
curl -k -u dumbmonit@example.com -H 'Content-Type: text/xml' --data '<MDaemon><API><Request version="20.0.0" echo="0" verbose="0"><Operation>GetVersionInfo</Operation><Parameters/></Request></API></MDaemon>' https://mail.example.com:444/MdMgmtWS/
```

Le Dynamic Screening peut bloquer une adresse après des échecs de connexion répétés : un mauvais mot de passe ici peut faire bloquer l’hôte DumbMonit, testez d’abord avec la commande ci-dessus. L’administration à distance utilise souvent un certificat autosigné : si la connexion est refusée pour cette raison, cochez « Accepter un certificat non vérifiable » dans les options.

!!! warning
    Les files, les sessions et les services Windows de MDaemon ne se lisent que sur le serveur lui-même : sans l’agent, DumbMonit voit les ports mail et rien d’autre. Le compte de l’API XML est facultatif et n’apporte que la version ; MDaemon n’a pas de rôle en lecture seule pour lui, ne réutilisez donc jamais votre compte, et prenez un mot de passe qui ne sert nulle part ailleurs, car des échecs répétés peuvent faire bloquer DumbMonit.

L’API XML est servie par le serveur web de Remote Administration : port
1000 en HTTP et 444 en HTTPS par défaut, sur `/MdMgmtWS/`, ou par IIS quand
Remote Administration y tourne. Sa référence complète est fournie avec le
serveur : ouvrez `https://mail.example.com:444/MdMgmtWS` dans un
navigateur, ou cherchez dans `\MDaemon\Docs\API\XML API\`.

## Identifiants

| Identifiant | Champs |
|---|---|
| Services seulement | Rien : les ports mail sont vérifiés sans se connecter. C’est le choix habituel : l’agent apporte le reste. |
| Compte API XML (facultatif) | L’adresse e-mail complète du compte, et son mot de passe, envoyés en authentification HTTP basique. Il n’ajoute que la version exacte. |

Adresse : le nom d’hôte ou l’IP du serveur, par exemple
`mail.example.com`. Un port écrit dans l’adresse
(`mail.example.com:8444`) est pris comme le port de l’API XML.

## Options

| Option | Défaut | Effet |
|---|---|---|
| `services` | `smtp,imap,webmail` | Quoi vérifier, séparé par des virgules : `smtp` (25), `msa` (587), `smtps` (465), `pop3` (110), `pop3s` (995), `imap` (143), `imaps` (993), `webmail` (3000), `remote_admin` (1000), `remote_admin_https` (444), `xmpp` (5222). `nom:port` change un port ; tout autre nom avec son port ajoute un service propre, vérifié par connexion seulement. `none` ne vérifie aucun port. |
| `agent_id` | aucun | La surveillance de l’agent installé sur le serveur MDaemon (choisie dans le formulaire ; l’id de la surveillance de l’agent). |
| `request_timeout_seconds` | `10` | Temps accordé à chaque connexion, salutation et appel d’API, de 1 à 60. |
| `api_port` | `444` | Le port Remote Administration qui sert `/MdMgmtWS/`. Utilisé seulement avec un compte. |
| `api_tls` | `true` | HTTPS vers l’API XML. Décochez seulement pour Remote Administration en HTTP simple (port 1000 par défaut) : le mot de passe traverse alors le réseau en clair. |
| `insecure_tls` | `false` | Accepter un certificat autosigné sur l’API XML. |

Les ports TLS (`smtps`, `pop3s`, `imaps`, `remote_admin_https`) sont
vérifiés par connexion seulement. Pour surveiller l’expiration de leurs
certificats, ajoutez une surveillance
[Certificat TLS](services.md#tls) pour chacun.

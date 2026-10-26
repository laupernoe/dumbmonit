# SecurityGateway for Email Servers

SecurityGateway se surveille avec **l’agent DumbMonit installé sur le serveur
Windows qui le fait tourner**. Ses services Windows, ses dossiers de files et
l’espace disque de ses données ne se lisent que sur le serveur lui-même.
L’agent les lit localement et les envoie à DumbMonit ; la surveillance
SecurityGateway relie ensuite l’agent à ce que DumbMonit vérifie depuis
l’extérieur : les ports de la passerelle et, sur SecurityGateway 12.5 ou plus
récent, la version et les compteurs de performance que publie son API REST
(file de livraison, quarantaine, sessions SMTP).

!!! info "Agent requis sur le serveur de messagerie"

    Sans l’[agent](agent.md), DumbMonit ne voit que les ports de la passerelle
    et, avec une clé d’API, les compteurs de l’API REST. La page de la
    surveillance le dit dans un bandeau : *Agent non installé ou non lié*, ou
    *Agent injoignable*. Ce bandeau est un avertissement sur la page, pas une
    panne : aucune notification n’est envoyée pour lui. La clé d’API reste
    recommandée : c’est elle qui apporte les compteurs de performance ;
    l’agent apporte le reste.

| | Avec l’agent sur le serveur | Sans lui |
|---|---|---|
| Ports de la passerelle (SMTP, interface web) | Vérifiés | Vérifiés |
| Services Windows de SecurityGateway (état, mode de démarrage) | Lus | Non surveillés |
| Dossiers de files de mail (messages et plus ancien message) | Lus | Non surveillés |
| Espace disque des données | Lu | Non surveillé |
| Compteurs de performance (file de livraison, quarantaine, sessions) et version | Avec une clé d’API | Avec une clé d’API |

## Ports réseau à ouvrir

De l’agent vers le serveur DumbMonit :

| Port | Protocole | Usage |
|---|---|---|
| 8080 | TCP | Mesures de l’agent (port par défaut, 443 derrière un proxy TLS) |

Du serveur DumbMonit (ou de son agent relais) vers l’équipement :

| Port | Protocole | Usage |
|---|---|---|
| 25 | TCP | Vérification SMTP (par défaut) |
| 4000 | TCP | Vérification de l’interface web (par défaut) |
| 4443 | TCP | Seulement avec une clé d’API (HTTPS, par défaut) ou web_https dans l’option Services |
| 465 | TCP | Seulement si smtps figure dans l’option Services |

L’agent appelle le serveur, jamais l’inverse : aucun port n’a besoin d’être
ouvert sur le serveur de la passerelle pour lui.

!!! warning "Validé contre la documentation seulement"

    Cette intégration a été construite à partir de la documentation publiée
    par MDaemon Technologies de SecurityGateway et de son API REST, pas
    contre une vraie passerelle. Chaque appel est une lecture. La
    documentation ne publie ni le chemin ni les noms des compteurs de
    performance, DumbMonit trouve donc le chemin dans la propre description
    de l’API et garde chaque compteur sous le nom que l’API lui donne. Les
    noms des services Windows de SecurityGateway et l’organisation de ses
    dossiers de files ne sont pas documentés non plus : l’agent cherche les
    services dont le nom contient `SecurityGateway` et compte les messages
    dans les sous-dossiers de `Queues` sous le dossier d’installation.
    Personne ne l’a encore fait tourner contre une passerelle en
    fonctionnement. Dites-nous ce qui casse.

## Installer

1. Installez l’[agent](agent.md#install) sur le serveur Windows qui fait
   tourner SecurityGateway : dans DumbMonit, ajoutez une surveillance
   **Serveur avec agent** et lancez sa commande PowerShell dans un PowerShell
   élevé. Depuis **Ajouter un équipement** → **SecurityGateway for Email
   Servers**, le bouton *Installer l’agent Windows* ouvre ce parcours avec la
   seule commande PowerShell, et le bouton *Ajouter la surveillance
   SecurityGateway* vous ramène ici.
2. Ajoutez la surveillance SecurityGateway et choisissez cet agent dans
   **Agent installé sur ce serveur**. Quand l’adresse saisie a le nom d’hôte
   d’un agent, DumbMonit le présélectionne.
3. Collez une clé d’API (recommandée, SecurityGateway 12.5 ou plus récent)
   pour lire les compteurs de performance. L’agent couvre le reste.

L’agent détecte SecurityGateway sans aucun réglage : il cherche un service
Windows dont le nom ou le nom affiché contient `SecurityGateway`, et prend le
dossier d’installation dans le chemin de son exécutable. Pour le désactiver,
réglez `mdaemon: false` dans `agent.yaml` (le même réglage couvre les deux
produits).

## Ce qu’il surveille

### Lu par l’agent {#read-by-the-agent}

Préfixé `dumbmonit_agent_mail_`, sur le `target` de l’agent, avec
`product="securitygateway"`.

| Métrique | Labels | Signification |
|---|---|---|
| `info` (valeur 1) | `product`, `version` | SecurityGateway a été trouvé sur le serveur, avec la version produit de son exécutable (absente si illisible). |
| `service_up` | `product`, `service`, `display`, `start` | 1 si le service Windows tourne. `start` est la façon dont Windows le démarre : `auto`, `manual`, `disabled` ou `other`. |
| `queue_files` | `product`, `queue` | Messages (fichiers `.msg`) dans chaque sous-dossier de `Queues` sous le dossier d’installation, nommé en minuscules. 20 dossiers au plus, et 20 000 fichiers par dossier. |
| `queue_oldest_seconds` | `product`, `queue` | Âge du plus ancien message d’un dossier de file non vide. |
| `data_free_bytes`, `data_total_bytes` | `product` | Espace libre et total du volume qui contient le dossier d’installation. |

Aucun compteur de performance Windows de SecurityGateway n’est lu par défaut,
puisque sa documentation ne nomme pas d’objet. Si votre serveur en publie,
listez-les sous `perf_counters` dans `agent.yaml` (voir
[Compteurs de performance Windows](agent.md#windows-performance-counters)).

### Services réseau et API

Vérifiés par DumbMonit depuis l’extérieur. Toutes les métriques sont
préfixées `dumbmonit_securitygateway_`.

| Métrique | Labels | Signification |
|---|---|---|
| `service_up` | `service`, `port` | 1 si le port a accepté la connexion et, pour SMTP, a salué avec `220`. 0 s’il n’a pas répondu, ou a répondu pour refuser le service (`421`, `554`). |
| `service_response_seconds` | `service`, `port` | Temps jusqu’à la salutation, ou jusqu’à la connexion pour un port qui n’en a pas. |
| `api_up` | | Avec une clé d’API seulement : 1 si l’API REST a répondu, 0 si elle n’a pas répondu. |
| `api_response_seconds` | | Temps que l’API REST a pris pour répondre. |
| `counters_available` | | 1 si les compteurs de performance ont été lus ; 0 si l’API n’en décrit aucun, ou si le compte de la clé ne peut pas les lire. |
| `counter` | `counter` | Une série par valeur numérique des compteurs de performance, nommée comme l’API la nomme, en `snake_case` : `deliveryQueue` devient `delivery_queue`. 64 au plus. |
| `info` (valeur 1) | `version`, `source` | La version donnée par la description de l’API (`source="openapi"`), ou sinon par la salutation SMTP quand la passerelle l’y annonce (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Temps que toute la vérification a pris. |

**Comment les compteurs sont trouvés.** L’API REST de SecurityGateway se
décrit elle-même à `/api/v1/openapi`. DumbMonit lit cette description, prend
le chemin en lecture seule le plus court sans paramètres dont le nom
mentionne des compteurs, le lit, et garde chaque nombre dans sa réponse. Rien
n’est renommé et rien n’est supposé : si une version future renomme un
compteur, la série suit le nouveau nom. Les compteurs listés dans l’aide de
SecurityGateway sont les sessions SMTP entrantes et sortantes actives, la
file de livraison, la quarantaine administrative, l’uptime, les compteurs de
domaines et d’utilisateurs et le pool de connexions de la base de données ;
certains ne changent qu’une fois par minute.

**Une panne partielle reste une vérification réussie.** Un service arrêté
donne `service_up 0` pour ce service et les autres sont toujours mesurés.
C’est seulement quand aucun service et aucune API ne répondent du tout que
la surveillance est *inaccessible*. Une clé refusée est une erreur de
configuration, affichée sur la page de la surveillance et non notifiée.

**Ce qui n’est pas lu.** La licence et son expiration, et les statistiques de
messages du tableau de bord, ne sont pas décrites par la documentation
publiée de l’API REST.

Les [règles intégrées](../alerting/rules.md#mdaemon-and-securitygateway) qui
s’appliquent : service SecurityGateway arrêté, API SecurityGateway ne répond
pas et file de livraison SecurityGateway qui grandit (depuis la
surveillance), plus Surveillance inaccessible ; et, sur la surveillance de
l’agent, Service Windows du serveur mail arrêté.

## La page de la surveillance

La page d’une surveillance SecurityGateway montre, de haut en bas :

1. Un **bandeau** quand l’agent est absent ou silencieux (*Agent non installé
   ou non lié*, *Agent injoignable*, ou *L’agent ne voit pas SecurityGateway
   sur sa machine*). C’est un avertissement, pas une alerte.
2. **Lu par l’agent** : les *Services Windows*, les *Files* (messages par
   dossier et âge du plus ancien), la version et l’espace disque.
3. **Services réseau** : chaque port surveillé, un mot et une couleur (Répond,
   Arrêté) et son temps de réponse ; puis, sous **API**, l’état de l’API
   REST, la version, et avec une clé d’API, chaque compteur de performance
   avec sa dernière valeur.

Tout est lu depuis la dernière mesure stockée : ouvrir la page ne se connecte
jamais à la passerelle.

## Installer l’agent, puis ajouter la surveillance

1. Sur le serveur Windows qui fait tourner SecurityGateway, ajoutez une surveillance Serveur avec agent dans DumbMonit et lancez sa commande PowerShell dans un PowerShell élevé. L’agent trouve SecurityGateway tout seul.

2. De retour ici, saisissez l’adresse de la passerelle et choisissez cet agent dans Agent installé sur ce serveur. DumbMonit présélectionne celui qui a le même nom d’hôte.

3. Recommandé, pour les compteurs de performance : dans SecurityGateway 12.5 ou plus récent, ouvrez Setup/Users → Accounts → Administrators → New et créez le compte ci-dessous, avec un long mot de passe unique.

    ```
    dumbmonit
    ```

4. Donnez au compte le rôle listé ci-dessous, et restreignez-le à l’adresse de DumbMonit dans IP Restriction Options.

5. Connectez-vous avec ce compte, ouvrez Setup/Users → Accounts → API Keys et créez une clé avec une date d’expiration, puis collez la clé dans DumbMonit.

6. Pour surveiller d’autres ports que SMTP et l’interface web, réglez l’option Services sur une liste comme celle-ci.

    ```
    smtp,web_https
    ```

### Droits requis

- **Administrateur local du serveur** : Seulement pour installer l’agent, qui tourne ensuite comme service Windows
- **Domain Administrator** : Prenez Global Administrator si les compteurs restent vides

Documentation de l’éditeur : <https://help.mdaemon.com/SecurityGateway/en/api_keys.html>

### Bon à savoir

Les services Windows, les dossiers de files et l’espace disque sont lus par l’agent, avec les droits du service Windows qui le fait tourner (Local System par défaut). Rien n’est écrit sur le serveur.

Le rôle est Domain Administrator ; si les compteurs restent vides sur la page de la surveillance, votre version les réserve au rôle Global Administrator, et le compte a besoin de ce rôle à la place. Une clé porte les droits du compte qui la crée, c’est pourquoi elle doit être créée depuis le compte dédié et pas depuis le vôtre. Les clés d’API respectent les restrictions IP du compte.

L’API est atteinte sur le port HTTPS de l’interface web, 4443 par défaut, sous `/api/v1`. Pour vérifier depuis l’hôte DumbMonit que la clé fonctionne, lancez la commande ci-dessous : elle affiche la description de l’API, qui liste aussi les compteurs que DumbMonit lit.

```
curl -k -H 'Authorization: Bearer YOUR_KEY' https://sg.example.com:4443/api/v1/openapi
```

L’interface web utilise souvent un certificat autosigné sur 4443 : si la connexion est refusée pour cette raison, cochez « Accepter un certificat non vérifiable » dans les options.

!!! warning
    Les services Windows, les dossiers de files de mail et l’espace disque de SecurityGateway ne se lisent que sur le serveur lui-même : sans l’agent, DumbMonit voit les ports de la passerelle et, avec une clé, les compteurs de l’API REST. SecurityGateway n’a ni rôle en lecture seule ni clé plus restreinte : créez la clé depuis le compte dédié, jamais depuis le vôtre, et donnez-lui une date d’expiration.

## Identifiants

| Identifiant | Champs |
|---|---|
| Services seulement | Rien : les ports sont vérifiés sans se connecter. L’agent apporte quand même les services Windows, les dossiers de files et l’espace disque. |
| Clé d’API (12.5 et plus récent, recommandée) | La clé, envoyée comme `Authorization: Bearer …`. Elle ajoute la version et les compteurs de performance (file de livraison, quarantaine, sessions). |

Adresse : le nom d’hôte ou l’IP de la passerelle, par exemple
`sg.example.com`. Un port écrit dans l’adresse (`sg.example.com:8443`) est
pris comme le port de l’API.

## Options

| Option | Défaut | Effet |
|---|---|---|
| `services` | `smtp,web` | Quoi vérifier, séparé par des virgules : `smtp` (25), `smtps` (465), `web` (4000), `web_https` (4443). `nom:port` change un port ; tout autre nom avec son port ajoute un service propre, vérifié par connexion seulement. `none` ne vérifie aucun port. |
| `agent_id` | aucun | La surveillance de l’agent installé sur le serveur SecurityGateway (choisie dans le formulaire ; l’id de la surveillance de l’agent). |
| `request_timeout_seconds` | `10` | Temps accordé à chaque connexion, salutation et appel d’API, de 1 à 60. |
| `api_port` | `4443` | Le port de l’interface web qui sert `/api/v1`. Utilisé seulement avec une clé d’API. |
| `api_tls` | `true` | HTTPS vers l’API REST. Décochez seulement pour l’interface web en HTTP simple (port 4000 par défaut) : la clé traverse alors le réseau en clair. |
| `insecure_tls` | `false` | Accepter un certificat autosigné sur l’API REST. |
| `counters` | `true` | Lire les compteurs de performance. |

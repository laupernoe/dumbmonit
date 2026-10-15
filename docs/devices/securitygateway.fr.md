# SecurityGateway for Email Servers

Les services d'une passerelle mail SecurityGateway, vérifiés depuis
l'extérieur, et sur SecurityGateway 12.5 ou plus récent, la version et les
compteurs de performance que publie son API REST : file de livraison,
quarantaine, sessions SMTP.

!!! warning "Validé contre la documentation seulement"

    Cette intégration a été construite à partir de la documentation publiée
    par MDaemon Technologies de SecurityGateway et de son API REST, pas
    contre une vraie passerelle. Chaque appel est une lecture. La
    documentation ne publie ni le chemin ni les noms des compteurs de
    performance, DumbMonit trouve donc le chemin dans la propre description
    de l'API et garde chaque compteur sous le nom que l'API lui donne.
    Personne ne l'a encore fait tourner contre une passerelle en
    fonctionnement. Dites-nous ce qui casse.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_securitygateway_`.

| Métrique | Labels | Signification |
|---|---|---|
| `service_up` | `service`, `port` | 1 si le port a accepté la connexion et, pour SMTP, a salué avec `220`. 0 s'il n'a pas répondu, ou a répondu pour refuser le service (`421`, `554`). |
| `service_response_seconds` | `service`, `port` | Temps jusqu'à la salutation, ou jusqu'à la connexion pour un port qui n'en a pas. |
| `api_up` | | Avec une clé d'API seulement : 1 si l'API REST a répondu, 0 si elle n'a pas répondu. |
| `api_response_seconds` | | Temps que l'API REST a pris pour répondre. |
| `counters_available` | | 1 si les compteurs de performance ont été lus ; 0 si l'API n'en décrit aucun, ou si le compte de la clé ne peut pas les lire. |
| `counter` | `counter` | Une série par valeur numérique des compteurs de performance, nommée comme l'API la nomme, en `snake_case` : `deliveryQueue` devient `delivery_queue`. 64 au plus. |
| `info` (valeur 1) | `version`, `source` | La version donnée par la description de l'API (`source="openapi"`), ou sinon par la salutation SMTP quand la passerelle l'y annonce (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Temps que toute la vérification a pris. |

**Comment les compteurs sont trouvés.** L'API REST de SecurityGateway se
décrit elle-même à `/api/v1/openapi`. DumbMonit lit cette description, prend
le chemin en lecture seule le plus court sans paramètres dont le nom
mentionne des compteurs, le lit, et garde chaque nombre dans sa réponse. Rien
n'est renommé et rien n'est supposé : si une version future renomme un
compteur, la série suit le nouveau nom. Les compteurs listés dans l'aide de
SecurityGateway sont les sessions SMTP entrantes et sortantes actives, la
file de livraison, la quarantaine administrative, l'uptime, les compteurs de
domaines et d'utilisateurs et le pool de connexions de la base de données ;
certains ne changent qu'une fois par minute.

**Une panne partielle reste une vérification réussie.** Un service arrêté
donne `service_up 0` pour ce service et les autres sont toujours mesurés.
C'est seulement quand aucun service et aucune API ne répondent du tout que
la surveillance est *inaccessible*. Une clé refusée est une erreur de
configuration, affichée sur la page de la surveillance et non notifiée.

**Ce qui n'est pas lu.** La licence et son expiration, l'espace disque et les
statistiques de messages du tableau de bord ne sont pas décrits par la
documentation publiée de l'API REST. Pour surveiller le propre service
Windows de SecurityGateway et les disques du serveur, installez l'[agent](agent.md)
sur le serveur et listez le service dans son réglage `services`.

Les [règles intégrées](../alerting/rules.md#mdaemon-and-securitygateway) qui
s'appliquent : service SecurityGateway arrêté, API SecurityGateway ne répond
pas, file de livraison SecurityGateway qui grandit, plus Surveillance
inaccessible.

## La page de la surveillance

Une surveillance SecurityGateway montre un panneau **Services mail** au-dessus
des graphiques, lu depuis la dernière mesure stockée : ouvrir la page ne se
connecte jamais à la passerelle. Il liste chaque service surveillé avec son
port, un mot et une couleur (Répond, Arrêté) et son temps de réponse, la
version, et avec une clé d'API, chaque compteur de performance avec sa
dernière valeur.

## Surveiller les services de la passerelle, et optionnellement l'API REST

1. Rien à installer sur le serveur. Sans identifiant, DumbMonit se connecte à SMTP (25) et à l'interface web (4000) depuis l'extérieur et lit la salutation SMTP. Changez la liste dans l'option Services, par exemple pour vérifier HTTPS plutôt que HTTP.

    ```
    smtp,web_https
    ```

2. Pour aussi lire les compteurs de performance, SecurityGateway 12.5 ou plus récent est nécessaire. Créez un compte dédié nommé comme suit, avec un mot de passe long utilisé nulle part ailleurs, et donnez-lui le rôle Domain Administrator. Si les compteurs restent vides sur la page de la surveillance, votre version les réserve au rôle Global Administrator, et le compte a besoin de ce rôle à la place.

    ```
    dumbmonit
    ```

3. Connectez-vous comme ce compte et créez une clé sous Setup/Users → Accounts → API Keys, avec une date d'expiration. Une clé porte les droits du compte qui la crée, c'est pourquoi elle doit être créée depuis ce compte et pas depuis le vôtre.

4. Restreignez le compte à l'adresse depuis laquelle DumbMonit se connecte dans ses restrictions IP : les clés d'API les respectent aussi.

5. Vérifiez depuis l'hôte DumbMonit que la clé fonctionne. La commande affiche la description de l'API, qui liste aussi les compteurs que DumbMonit lit.

    ```
    curl -k -H 'Authorization: Bearer YOUR_KEY' https://sg.example.com:4443/api/v1/openapi
    ```

6. Dans DumbMonit, saisissez l'adresse de la passerelle, par exemple « sg.example.com », et collez la clé. L'API est atteinte sur le port HTTPS de l'interface web, 4443 par défaut, sous /api/v1.

!!! warning
    SecurityGateway n'a pas de rôle en lecture seule ni de clé plus restreinte : la clé peut tout faire que son compte peut faire. Ne la créez jamais depuis votre propre compte, gardez-la seulement dans DumbMonit et donnez-lui une date d'expiration. Les versions avant 12.5 n'ont pas d'API REST : laissez l'identifiant vide et DumbMonit vérifie seulement les services. L'interface web utilise souvent un certificat autosigné sur 4443 : si la connexion est refusée pour cette raison, cochez « Accepter un certificat non vérifiable » dans les options.

## Identifiants

| Identifiant | Champs |
|---|---|
| Services seulement | Rien : les ports sont vérifiés sans se connecter. |
| Clé d'API (12.5 et plus récent) | La clé, envoyée comme `Authorization: Bearer …`. |

Adresse : le nom d'hôte ou l'IP de la passerelle, par exemple
`sg.example.com`. Un port écrit dans l'adresse (`sg.example.com:8443`) est
pris comme le port de l'API.

## Options

| Option | Défaut | Effet |
|---|---|---|
| `services` | `smtp,web` | Quoi vérifier, séparé par des virgules : `smtp` (25), `smtps` (465), `web` (4000), `web_https` (4443). `nom:port` change un port ; tout autre nom avec son port ajoute un service propre, vérifié par connexion seulement. `none` ne vérifie aucun port. |
| `request_timeout_seconds` | `10` | Temps accordé à chaque connexion, salutation et appel d'API, de 1 à 60. |
| `api_port` | `4443` | Le port de l'interface web qui sert `/api/v1`. Utilisé seulement avec une clé d'API. |
| `api_tls` | `true` | HTTPS vers l'API REST. Décochez seulement pour l'interface web en HTTP simple (port 4000 par défaut) : la clé traverse alors le réseau en clair. |
| `insecure_tls` | `false` | Accepter un certificat autosigné sur l'API REST. |
| `counters` | `true` | Lire les compteurs de performance. |

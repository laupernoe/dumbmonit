# mailcow (beta)

Si le serveur de messagerie livre : conteneurs arrêtés, taille et ancienneté de la file de Postfix, volume qui contient les boîtes et boîtes proches de leur quota.

!!! note "Bêta"
    Écrit d'après la documentation d'API de l'éditeur et testé sur des réponses réalistes enregistrées, pas encore sur une instance réelle. Dites-nous ce que vous observez.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | API (HTTPS, par défaut) |
| 80 | TCP | Seulement avec l'option de schéma http |

La panne à surveiller : **un conteneur arrêté dans une pile de vingt**. rspamd tombé, c'est plus de filtrage antispam, Dovecot tombé plus de lecture, Postfix tombé plus de livraison, et l'interface web s'ouvre toujours. Une boîte à son quota voit son courrier entrant refusé sans que l'expéditeur ne le sache.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_mailcow_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `containers_total`, `containers_running` | conteneurs mailcow, et ceux qui tournent |  |
| `container_up` | 1 par conteneur tant qu'il tourne (32 au plus) | `container` |
| `vmail_used_percent` | le volume qui contient les boîtes |  |
| `queue_messages`, `queue_oldest_seconds` | messages dans la file de Postfix et ancienneté du plus ancien |  |
| `mailboxes_total`, `mailboxes_inactive`, `mailboxes_near_quota` | boîtes ; boîtes désactivées ; boîtes à 90 % de leur quota ou plus |  |
| `mailbox_quota_percent` | boîtes à 80 % de leur quota ou plus, 32 au plus | `mailbox` |
| `scrape_errors` | appels qui ont échoué à la dernière sonde |  |

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui s'appliquent : mailcow container down, mailcow mail queue stuck, mailcow mail volume almost full, mailcow mailbox near quota, plus Appareil inaccessible.

## Créer une clé d'API en lecture seule dans mailcow

1. Dans mailcow, ouvrez System → Configuration → Access → API et repérez la section Read-only access.

2. Cochez Activate API, ajoutez l'adresse IP de DumbMonit aux adresses autorisées et enregistrez.

3. Copiez la clé en lecture seule, puis saisissez l'adresse de mailcow dans DumbMonit et collez-la.

!!! warning
    mailcow ne répond qu'aux adresses autorisées sur sa page d'API : si DumbMonit l'atteint à travers un NAT ou un agent relais, autorisez plutôt cette adresse.

## Identifiants

| Identifiant | Champs |
|---|---|
| Clé d'API | Envoyée dans l'en-tête `X-API-Key`. |

Adresse : un nom d'hôte (`mail.example.com`), `host:port`, ou une URL complète. Les noms de boîtes ne sont publiés que pour celles proches de leur quota. Options : protocole (HTTPS par défaut), port (443), vérification du certificat et délai de requête (15 s).

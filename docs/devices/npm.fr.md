# Nginx Proxy Manager

Le gestionnaire de proxy : les hôtes dont nginx a refusé la configuration, les
hôtes désactivés, et les certificats que les hôtes présentent, avant qu'un
échec de renouvellement Let's Encrypt ne tourne en site expiré.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 81 | TCP | API d'administration (HTTP, par défaut) |
| 443 | TCP | Seulement avec la vérification des certificats (port HTTPS des hôtes, par défaut) |

DumbMonit se connecte à l'API REST de Nginx Proxy Manager (`/api`, sur le port
de son interface web, 81) avec un utilisateur qui lui est propre, puis lit les
hôtes proxy, les hôtes de redirection, les hôtes 404 (appelés dead hosts dans
l'API) et les flux. Chaque hôte dit s'il est activé et si nginx a accepté sa
configuration. Un hôte que nginx a refusé n'est pas servi, et l'interface web
ne le montre que par un point rouge.

Les certificats ne sont pas lus depuis l'API. `/api/nginx/certificates` donne
la clé privée des certificats importés, et les identifiants du défi DNS, à
tout utilisateur autorisé à voir les certificats. DumbMonit lit chaque
certificat là où il est servi à la place : une poignée de main TLS sur le
port 443 de Nginx Proxy Manager, avec le nom d'un hôte qui l'utilise. Cela
mesure aussi le certificat que les visiteurs obtiennent réellement.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_npm_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `hosts`, `hosts_enabled`, `hosts_disabled` | hôtes de chaque type | `type` : `proxy`, `redirection`, `dead` (hôtes 404), `stream` |
| `hosts_offline` | hôtes activés dont nginx a refusé la configuration | `type` |
| `host_online` | par hôte activé : 0 quand nginx a refusé sa configuration | `type`, `host` |
| `host_error_info` | valeur 1, pour un hôte que nginx a refusé : la première ligne de l'erreur de nginx | `type`, `host`, `error` |
| `certificates_checked`, `certificates_read` | certificats attachés à un hôte activé, et ceux lus sur le port HTTPS | |
| `cert_expiry_days` | jours restants avant l'expiration du certificat qu'un hôte présente | `host` |

`host` est le premier nom de domaine de l'hôte, ou `port 9000` pour un flux.
Les hôtes désactivés sont comptés, pas décrits : désactiver un hôte est
délibéré, et son alerte, s'il y en a une, se résout. Les hôtes sont décrits un
par un jusqu'à 500, et 100 certificats au plus sont lus par vérification.

Les [règles intégrées](../alerting/rules.md#reverse-proxies-and-domains) qui
s'appliquent : hôte Nginx Proxy Manager hors ligne, certificat de proxy non
renouvelé, plus Surveillance inaccessible.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais Nginx Proxy Manager. Il dit ce qui ne va pas en une
phrase (hôtes que nginx a refusés, avec sa raison, certificats proches de
l'expiration, certificats qui n'ont pas pu être lus), puis les hôtes proxy,
redirections, hôtes 404, flux et hôtes désactivés, puis les certificats les
plus proches de l'expiration en premier.

## Créer un utilisateur Nginx Proxy Manager en lecture seule

1. Dans Nginx Proxy Manager, ouvrez Users → Add User, donnez-lui un e-mail qui lui est propre, comme dumbmonit@example.com, et laissez tous les rôles décochés.

2. Depuis le menu de cet utilisateur, ouvrez Permissions et réglez les niveaux listés ci-dessous.

3. Depuis le même menu, choisissez Change Password et définissez un mot de passe.

4. Dans DumbMonit, saisissez l'hôte Nginx Proxy Manager avec son port web, par exemple npm.lan:81, ainsi que cet e-mail et ce mot de passe.

### Droits requis

- **Item Visibility: All Items** : Voir tous les hôtes, pas seulement les siens
- **Proxy Hosts, Redirection Hosts, 404 Hosts, Streams: View Only** : Lire la liste des hôtes
- **Access Lists, SSL Certificates: Hidden** : Garde les clés privées hors de portée

!!! warning
    Laissez SSL Certificates sur Hidden : Nginx Proxy Manager remet les clés privées à tout utilisateur autorisé à les voir.

### Pourquoi ces droits

Les certificats SSL restent masqués volontairement : Nginx Proxy Manager remet la clé privée des certificats importés, et les identifiants du challenge DNS, à tout utilisateur autorisé à les voir. DumbMonit lit plutôt chaque certificat là où il est servi, par une poignée de main TLS sur le port 443 pour chaque hôte qui en a un ; si les hôtes sont servis en HTTPS sur un autre port, renseignez Port HTTPS dans le formulaire. Un hôte dont la configuration est refusée par nginx (faute de frappe dans Advanced, fichier de certificat manquant) n'est pas servi, et Nginx Proxy Manager ne le montre que par un point rouge dans la liste. Les certificats Let's Encrypt sont renouvelés trente jours à l'avance : un certificat qui expire dans moins de quatorze jours est un renouvellement en échec, et DumbMonit avertit alors.

Documentation de l'éditeur : https://nginxproxymanager.com/advanced-config/

## Identifiants

| Identifiant | Champs |
|---|---|
| Email et mot de passe de l'utilisateur | L'adresse email et le mot de passe de l'utilisateur créé ci-dessus. |

Adresse : un nom d'hôte ou une IP avec le port de l'interface web
(`npm.lan:81`), ou une URL complète. Options : protocole (HTTP par défaut),
port de l'interface web (81), Vérifier les certificats (activé), port HTTPS
(443), vérification du certificat de l'API et délai de requête (10 s).

La vérification des certificats se connecte à la partie hôte de l'adresse :
quand l'interface web est atteinte via un autre proxy, pointez l'adresse sur
Nginx Proxy Manager lui-même, ou décochez Vérifier les certificats et
surveillez chaque site avec une
[vérification de certificat TLS](services.md) à la place. Les certificats
wildcard sont lus via le premier hôte qui les utilise.

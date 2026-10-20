# MikroTik RouterOS

Routeurs et commutateurs faisant tourner RouterOS 7 : si RouterOS et le
firmware RouterBOOT ont pris du retard, CPU, mémoire et stockage,
températures, ventilateurs et alimentations, et l'état, le trafic, les
erreurs et les pertes de lien de chaque interface.

DumbMonit lit l'API REST que RouterOS 7 sert sous `/rest`, avec un
utilisateur dans un groupe qui ne peut que lire. Il envoie des requêtes
`GET` et rien d'autre.

Un MikroTik casse rarement ; il vieillit. Une release RouterOS qui n'est
plus la dernière de son canal, un firmware RouterBOOT laissé en arrière
après une mise à jour (il ne suit que lorsqu'on le lui demande, au prochain
redémarrage), une alimentation redondante morte sans que personne ne le
remarque, un port qui continue de journaliser des erreurs ou de perdre son
lien. C'est ce que cette page surveille, avec la charge.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_mikrotik_`. Les totaux sont
des compteurs : graphiques et règles les transforment en débits.

| Métrique | Quoi | Labels |
|---|---|---|
| `info` | valeur 1 | `version`, `release` (`long-term`, `stable`…), `board`, `architecture`, `identity` |
| `uptime_seconds` | temps depuis le dernier démarrage | |
| `cpu_load_percent`, `cpu_count` | charge CPU, tous cœurs | |
| `memory_total_bytes`, `memory_free_bytes`, `memory_used_percent` | mémoire | |
| `storage_total_bytes`, `storage_free_bytes`, `storage_used_percent` | la flash ou le disque sur lequel RouterOS vit | |
| `update_checked` | 1 quand le routeur a vérifié une version plus récente de RouterOS et obtenu une réponse | |
| `update_available` | 1 quand la dernière version du canal diffère de celle installée | `installed_version`, `latest_version`, `channel` |
| `routerboard` | 1 sur du matériel RouterBOARD, 0 sur un Cloud Hosted Router ou une machine x86 | |
| `firmware_upgrade_pending` | 1 quand RouterBOOT est plus ancien que le firmware fourni avec RouterOS | `current_firmware`, `upgrade_firmware`, `model` |
| `temperature_celsius`, `voltage_volts`, `fan_rpm`, `power_watts`, `current_amperes` | chaque capteur que la carte a | `sensor` (`cpu-temperature`, `fan1-speed`…) |
| `health_ok` | 1 quand un capteur `psu*-state` ou `fan*-state` lit `ok`, 0 sinon | `sensor` |
| `interface_running` | 1 quand l'interface a un lien | `interface`, `type` |
| `interface_rx_bytes_total`, `interface_tx_bytes_total`, `interface_rx_packets_total`, `interface_tx_packets_total` | trafic | `interface`, `type` |
| `interface_rx_errors_total`, `interface_tx_errors_total`, `interface_rx_drops_total`, `interface_tx_drops_total` | erreurs et pertes | `interface`, `type` |
| `interface_link_downs_total` | nombre de fois où le lien a été perdu | `interface`, `type` |
| `interfaces_skipped` | interfaces laissées de côté au-delà de la limite réglée dans les options | |
| `scrape_errors` | lectures secondaires qui ont échoué pendant la dernière sonde | |

Seules les interfaces activées sont lues. Les dynamiques (un client PPPoE
ou L2TP, dont le nom change à chaque connexion) et la loopback sont laissées
de côté.

Les capteurs dépendent de la carte : un hAP a une tension et une
température, un CCR ajoute des ventilateurs et des alimentations, un Cloud
Hosted Router n'en a aucun. Un capteur que la carte n'a pas ne produit
aucune série.

La version la plus récente de RouterOS n'est connue que quand le routeur
lui-même vérifie les mises à jour : un utilisateur en lecture seule n'a pas
le droit de démarrer cette vérification. L'étape optionnelle ci-dessous la
planifie sur le routeur. Sans elle, `update_checked` reste à 0 et la règle
de mise à jour ne se déclenche jamais.

Les [règles intégrées](../alerting/rules.md#mikrotik-routeros) qui
s'appliquent : CPU MikroTik élevé, mémoire élevée, stockage presque plein,
température élevée, alimentation ou ventilateur en échec, interface
accumulant des erreurs, lien instable, mise à jour RouterOS disponible et
mise à jour de firmware en attente, plus Surveillance inaccessible.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page ne se connecte jamais au routeur. Il montre la version de RouterOS et
si une plus récente est disponible, le firmware RouterBOOT, l'uptime, le
CPU, la mémoire et le stockage, chaque capteur avec son état, et un tableau
des interfaces avec leur lien, trafic, erreurs et pertes de lien.

## Créer un utilisateur en lecture seule sur le routeur

1. WinBox ou WebFig → System → Users → Groups → Add : nommez le groupe comme suit et cochez uniquement les policies listées ci-dessous.

    ```
    dumbmonit-read
    ```

2. System → Users → Add : créez l'utilisateur dumbmonit dans ce groupe, avec un long mot de passe aléatoire.

    ```
    dumbmonit
    ```

3. IP → Services : activez www-ssl et choisissez un certificat (créez-en un sous System → Certificates).

4. Facultatif, pour être prévenu des nouvelles versions de RouterOS : System → Scheduler → Add, interval 1d, start time startup, avec ce script On Event.

    ```
    /system package update check-for-updates once
    ```

5. Dans DumbMonit, saisissez l'adresse du routeur (192.168.88.1) avec ce nom d'utilisateur et son mot de passe.

### Droits requis

- **read** — Lire l'état du routeur
- **api** — Ouvrir l'API
- **rest-api** — Ouvrir l'API REST sous /rest

!!! warning
    RouterOS 7.1 ou plus est requis (RouterOS 6 n'a pas d'API REST) ; avec un certificat auto-signé, cochez « Accept an unverifiable certificate » dans les options.

Documentation de l'éditeur : https://help.mikrotik.com/docs/spaces/ROS/pages/47579162/REST+API

### Pourquoi ces droits

RouterOS exige les trois policies pour une lecture REST : `rest-api` ouvre
l'API, `api` et `read` autorisent la lecture elle-même. Sans `rest-api` le
routeur répond 401, sans `api` « not allowed », sans `read` « not enough
permissions » ; DumbMonit signale chaque cas comme un problème d'identifiants
qui nomme la policy manquante. Avec les trois, un changement, un redémarrage ou
une vérification de mise à jour est refusé. N'utilisez jamais le compte par
défaut, qui a tous les droits. Pour n'accepter le compte que depuis DumbMonit,
réglez l'Allowed Address de l'utilisateur sur l'hôte DumbMonit.

RouterOS livre le service HTTPS désactivé et sans certificat. Vous préférez le
terminal ? Ces commandes créent un certificat sur le routeur lui-même (cochez
ensuite « Accept an unverifiable certificate » dans les options) :

```
/certificate add name=local-ca common-name=local-ca key-usage=key-cert-sign,crl-sign days-valid=3650
/certificate sign local-ca
/certificate add name=https-cert common-name=router.lan days-valid=825
/certificate sign https-cert ca=local-ca
/ip service set www-ssl certificate=https-cert disabled=no
```

Le service `www` (HTTP en clair) répond aussi à l'API REST, mais envoie le mot
de passe en clair à chaque requête. Un utilisateur en lecture seule ne peut pas
lancer la vérification de mise à jour : DumbMonit n'en lit que le résultat.

Les trois politiques ont été vérifiées une par une sur RouterOS 7.23.7,
chacune avec un utilisateur neuf : sans `rest-api` le routeur répond 401,
sans `api` il répond « not allowed », sans `read` « not enough
permissions ». DumbMonit rapporte chaque cas comme un problème d'identifiant
nommant la politique manquante, pas comme un routeur en panne. Avec les
trois, un changement, un redémarrage ou une vérification de mise à jour est
refusé avec « not enough permissions ».

RouterOS peut continuer d'appliquer les anciennes politiques d'un groupe à
un utilisateur un moment après que le groupe a été modifié. Si vous changez
les politiques d'un groupe existant et que l'erreur persiste, supprimez et
recréez l'utilisateur.

## Identifiants

| Identifiant | Champs |
|---|---|
| Utilisateur en lecture seule | L'utilisateur `dumbmonit` et son mot de passe, envoyés en authentification HTTP basique à chaque requête. L'API REST n'a pas de session. |

Adresse : un nom d'hôte ou une IP (`192.168.88.1`), `host:port`, ou une URL
complète (`https://router.lan:8443`). Options : protocole (HTTPS par
défaut ; HTTP utilise le service `www`), port (443 pour HTTPS, 80 pour
HTTP), vérification du certificat, délai de requête (10 s), si les
interfaces doivent être lues (activé), et combien au plus (64).

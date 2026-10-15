# pfSense

Un pare-feu pfSense via le paquet communautaire d'API REST
(`pfSense-pkg-RESTAPI`, version 2) : passerelles en panne, qui perdent des
paquets ou répondent lentement, interfaces activées sans lien, services
activés qui se sont arrêtés, et CPU, mémoire, disque et température.

DumbMonit envoie une clé d'API dans l'en-tête `X-API-Key`. Une clé porte
exactement les privilèges de l'utilisateur pfSense auquel elle appartient :
l'utilisateur `dumbmonit` ci-dessous détient seulement les privilèges `GET`
des cinq pages que DumbMonit lit, la clé peut donc lire ces pages et ne rien
changer. Chaque minute, DumbMonit lit, en parallèle,
`/api/v2/status/system`, `/api/v2/status/gateways`,
`/api/v2/status/interfaces`, `/api/v2/status/services` et
`/api/v2/system/version`, et, si le privilège a été donné,
`/api/v2/system/restapi/version`.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la documentation
    publiée du paquet d'API REST et du code source de ses modèles (noms de
    champs, unités, l'enveloppe de réponse et ses identifiants d'erreur),
    pas contre un pfSense en fonctionnement. Chaque appel est une lecture.
    Dites-nous ce qui casse.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_pfsense_`.

| Métrique | Labels | Signification |
|---|---|---|
| `cpu_usage_percent`, `memory_used_percent`, `swap_used_percent`, `disk_used_percent`, `mbuf_used_percent` | | Comme le tableau de bord les montre, en pourcentage. Le chiffre de CPU est la propre estimation de pfSense depuis la moyenne de charge. |
| `temperature_celsius` | | Quand pfSense peut lire un capteur. |
| `load_average` | `period` : `1m`, `5m`, `15m` | Moyenne de charge. |
| `cpu_count` | | Cœurs de processeur. |
| `version_info` | `version` | Valeur 1 : la version de pfSense, `2.7.2-RELEASE`. |
| `gateway_up` | `gateway` | 0 quand pfSense marque la passerelle en panne, 1 sinon. |
| `gateway_status_info` | `gateway`, `status` | Valeur 1 : le propre mot de pfSense (`none` quand tout va bien, `down`, `loss`, `delay`). |
| `gateway_loss_percent` | `gateway` | Perte de paquets vers l'adresse de surveillance. |
| `gateway_delay_milliseconds`, `gateway_stddev_milliseconds` | `gateway` | Aller-retour vers l'adresse de surveillance et son écart ; absent tant que la passerelle est en panne. |
| `gateways`, `gateways_down` | | Compteurs. |
| `interface_enabled`, `interface_up` | `interface`, `descr` | Activée dans la configuration ; lien actif. |
| `interface_down` | `interface`, `descr` | 1 quand l'interface est activée et n'a pas de lien. Une interface désactivée n'est jamais en panne. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `descr` | Compteurs. |
| `interfaces_down` | | Compteur. |
| `service_running`, `service_enabled` | `service`, `description` | Comme Status > Services les montre. |
| `service_stopped` | `service`, `description` | 1 quand le service est activé et ne tourne pas. |
| `services_stopped` | | Compteur. |
| `restapi_update_available` | | Avec le privilège `/api/v2/system/restapi/version` : 1 quand le paquet d'API REST a une release plus récente. |
| `restapi_version_info` | `version`, `latest` | Valeur 1 : release installée et dernière release du paquet. |

pfSense ne dit pas via cette API si une mise à jour de pfSense est
disponible : la seule route qui touche aux mises à jour système
(`/api/v2/system/update`) en démarre une. DumbMonit rapporte seulement les
propres mises à jour du paquet.

Les [règles intégrées](../alerting/rules.md#pfsense) qui s'appliquent :

- **Passerelle pfSense en panne** : une passerelle est marquée en panne
  depuis trois minutes (Avertissement).
- **Passerelle pfSense perd des paquets** : plus de 10 % de perte pendant
  dix minutes, sur une passerelle qui n'est pas déjà en panne (Avis).
- **Passerelle pfSense lente** : aller-retour au-dessus de 500 ms pendant
  quinze minutes (Avis). Augmentez-le pour une liaison satellite.
- **Interface pfSense sans lien** : une interface activée sans lien depuis
  cinq minutes (Avis).
- **Service pfSense arrêté** : un service activé qui ne tourne pas depuis
  cinq minutes (Avis).
- **Disque pfSense presque plein** : plus de 90 % pendant trente minutes
  (Avis).

## La page de la surveillance

Le panneau Firewall dit ce qui ne va pas en une phrase, montre le CPU, la
mémoire, le disque et la température, puis liste chaque passerelle avec sa
latence et sa perte, les interfaces sans lien, les services arrêtés, et une
mise à jour disponible du paquet d'API REST.

## Créer une clé d'API REST en lecture seule pour DumbMonit

1. Installez le paquet communautaire d'API REST (pfSense-pkg-RESTAPI, version 2) depuis Diagnostics > Command Prompt ou un shell SSH, avec le fichier de la page des releases du projet qui correspond à votre version de pfSense (2.7.2 ci-dessous). Puis, sous System > REST API > Settings, cochez à la fois BasicAuth et Key parmi les méthodes d'authentification.

    ```
    pkg-static add https://github.com/pfrest/pfSense-pkg-RESTAPI/releases/latest/download/pfSense-2.7.2-pkg-RESTAPI.pkg
    ```

2. Sous System > User Manager, ajoutez un utilisateur nommé dumbmonit avec un mot de passe long et aléatoire. Sous Effective Privileges, ajoutez seulement ces privilèges d'API REST : /api/v2/status/system GET, /api/v2/status/gateways GET, /api/v2/status/interfaces GET, /api/v2/status/services GET, /api/v2/system/version GET, plus /api/v2/system/restapi/version GET pour connaître les propres mises à jour du paquet. Pour l'étape suivante seulement, ajoutez aussi /api/v2/auth/key POST.

3. Créez la clé en tant que dumbmonit, depuis n'importe quelle machine qui atteint pfSense : la clé porte exactement les privilèges de cet utilisateur. Copiez la valeur de clé de la réponse, puis retirez à nouveau le privilège /api/v2/auth/key POST.

    ```
    curl -k -u dumbmonit -X POST -H 'Content-Type: application/json' -d '{"descr":"DumbMonit"}' https://pfsense.lan/api/v2/auth/key
    ```

4. Dans DumbMonit, saisissez l'adresse de pfSense, par exemple « pfsense.lan », et collez la clé comme Clé d'API. pfSense livre un certificat autosigné : cochez Accepter un certificat non vérifiable sauf si vous l'avez remplacé.

!!! warning
    L'API REST ne dit pas si une mise à jour de pfSense est disponible, seulement si le paquet lui-même en a une. Le paquet est un projet communautaire, pas une partie de pfSense : réinstallez-le après chaque mise à jour de pfSense.

## Identifiants

| Identifiant | Champs |
|---|---|
| Clé d'API REST | La clé créée pour `dumbmonit` à l'étape 3, envoyée comme `X-API-Key`. |

Adresse : le nom d'hôte ou l'IP de pfSense, `host:port`, ou une URL.
Options :

- **Protocole** (`https`) et **Port** (443) : ceux du webConfigurator,
  réglés sous System > Advanced > Admin Access.
- **Accepter un certificat non vérifiable** : le propre certificat de
  pfSense est autosigné.
- **Délai par requête** (15 s).

## Dépannage

« pfSense refused the API key (401) » : la clé est incorrecte, ou Key n'est
pas cochée parmi les méthodes d'authentification sous System > REST API >
Settings.

« pfSense accepted the API key but refused … (403) » : l'utilisateur
`dumbmonit` n'a pas le privilège de cette page ; le message le nomme.

« … not found: is the REST API package installed? » : le paquet est
manquant, typiquement après une mise à jour de pfSense, qui le supprime.
Réinstallez-le comme à l'étape 1.

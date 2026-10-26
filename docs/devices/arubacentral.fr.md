# Aruba Central

Les bornes Wi-Fi, commutateurs et passerelles d'un compte HPE Aruba
Networking Central, lus par l'API cloud de Central : appareils passés hors
ligne, alertes critiques actives, appareils en mauvaise santé et nombre de
clients connectés.

DumbMonit lit l'API de **new Central** (celle des passerelles
`*.api.central.arubanetworks.com`), une fois par minute. L'identifiant
recommandé est un client d'API personnel créé dans HPE GreenLake : DumbMonit
échange son identifiant et son secret contre un jeton d'accès de deux heures
(`https://sso.common.cloud.hpe.com/as/token.oauth2`, `client_credentials`) et
le garde jusqu'à son expiration. Un jeton d'accès collé à la main fonctionne
aussi, mais DumbMonit ne peut pas le renouveler : il cesse de marcher au bout
de deux heures.

C'est une API sur Internet : l'adresse du moniteur est la passerelle d'API de
votre région, pas une machine de votre réseau, et rien sur vos sites n'est
contacté. Chaque appel est une lecture.

!!! warning "Bêta"

    Écrit d'après la référence d'API publiée par HPE (`getDevicesV1`,
    `getAlertListV1`, `tenantClientHealthV1`, `tenantDeviceHealthV1`), pas
    encore testé sur un vrai compte Central. Dites-nous ce qui casse. Les
    identifiants limités à la plateforme HPE GreenLake (autre point d'accès
    pour les jetons) et Central classique (`/monitoring/v1/…`) ne sont pas
    pris en charge.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | API Central (HTTPS, l'URL de base de votre région) |
| 443 | TCP | Connexion HPE GreenLake (HTTPS, sso.common.cloud.hpe.com) |

## Ce qu'il lit

Une page requise et trois facultatives qu'un client privé du droit
correspondant peut refuser sans faire échouer le reste :

| Page | Usage |
|---|---|
| `GET /network-monitoring/v1/devices` (requise) | Tous les points d'accès, commutateurs et passerelles : nom, modèle, micrologiciel, adresse, site, `ONLINE` ou `OFFLINE`, durée de fonctionnement. Lus par mille, sur dix pages au plus. |
| `GET /network-notifications/v1/alerts` | Les alertes actives (les cent plus récentes sont classées par gravité). |
| `GET /network-monitoring/v1/tenant-client-health` | Clients par type (sans fil, filaire) et par santé. |
| `GET /network-monitoring/v1/tenant-device-health` | Appareils par type et par santé. |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_arubacentral_`.

| Métrique | Labels | Signification |
|---|---|---|
| `device_online` | `device`, `type`, `serial`, `site` | 1 quand Central voit l'appareil `ONLINE`. `type` vaut `access_point`, `switch` ou `gateway`. |
| `device_uptime_seconds` | `device`, `type`, `serial` | Durée de fonctionnement, tant qu'il est en ligne. |
| `device_info` | `device`, `type`, `serial`, `model`, `firmware`, `ip`, `site` | Valeur 1. |
| `devices`, `devices_offline` | | Nombre d'appareils, et de ceux hors ligne. |
| `devices_by_type`, `devices_offline_by_type` | `type` | Les mêmes comptes par type. |
| `alerts_active` | | Alertes actives dans Central. |
| `alerts_active_by_severity` | `severity` | Parmi les cent alertes actives les plus récentes ; `critical` est toujours publiée, à 0 s'il n'y en a pas. |
| `clients` | `type` | Clients connectés par type. |
| `clients_by_health` | `type`, `health` | Clients par groupe de santé (`good`, `fair`, `poor`). |
| `devices_by_health` | `type`, `health` | Appareils par groupe de santé. |
| `devices_poor_health` | | Appareils du groupe `poor`. |

Les [règles intégrées](../alerting/rules.md#aruba-central) qui s'appliquent :

- **Aruba Central device offline** : Central voit un appareil hors ligne
  pendant dix minutes (Avis).
- **Aruba Central critical alert** : au moins une alerte critique active
  pendant dix minutes (Avis).
- **Aruba Central devices in poor health** : au moins un appareil du groupe de
  santé `poor` pendant une heure (Info).

## Créer un client d'API personnel pour DumbMonit

1. Connectez-vous à HPE GreenLake (common.cloud.hpe.com) et ouvrez Manage Workspace → Personal API clients.

2. Cliquez sur Create personal API client, nommez-le comme suit et choisissez votre instance Central comme service.

    ```
    dumbmonit
    ```

3. Copiez l'identifiant (client ID) et le secret (client secret) : le secret n'est affiché qu'une fois, et HPE n'en garde aucune copie.

4. Dans Central, ouvrez la page de la passerelle d'API (Menu → API Gateway → REST API) et notez l'URL de base de votre région.

5. Dans DumbMonit, saisissez cette URL de base comme adresse (par exemple us1.api.central.arubanetworks.com) et collez le client ID et le client secret.

### Droits requis

- **Accès en lecture à la surveillance de Central** — Appareils, alertes et vues de santé : DumbMonit ne modifie jamais rien

!!! warning
    Bêta : écrit d'après la référence d'API publiée par HPE, pas encore testé sur un vrai compte Central. Il lit l'API de new Central avec des identifiants Central ; les identifiants limités à la plateforme GreenLake et Central classique ne sont pas pris en charge.

Documentation de l'éditeur : https://developer.arubanetworks.com/new-central/docs/generating-and-managing-access-tokens

### URL de base par région

L'adresse est la passerelle d'API du cluster qui héberge votre compte, telle
que HPE la liste : `us1.api.central.arubanetworks.com` (US-1), `us2.api…`
(US-2), `us4.api…` (US-WEST-4), `us5.api…` (US-WEST-5), `us6.api…`
(US-East1), `de1.api…` (EU-1), `de2.api…` et `de3.api…` (EU-Central2 et 3),
`gb1.api…` (Royaume-Uni), `ca1.api…` (Canada), `in1.api…` (APAC-1),
`jp1.api…` (APAC-EAST1), `au1.api…` (APAC-SOUTH1), `ae1.api…` (Émirats), tous
sous `central.arubanetworks.com`, et `cn1.api.central.arubanetworks.com.cn`
(Chine).

## Identifiants

| Identifiant | Champs |
|---|---|
| Client d'API | Client ID et client secret du client d'API personnel. |
| Jeton d'accès | Un jeton d'accès de votre instance Central (valable deux heures, non renouvelé). |

Options :

- **Sign-in host** (`sso.common.cloud.hpe.com`) : l'hôte qui délivre les
  jetons ; ne le changez que si HPE vous en indique un autre.
- **Timeout per request** (15 s).

## Dépannage

« Aruba Central refused the credentials (401) » : le jeton a expiré, ou le
client d'API a été réinitialisé ou supprimé. Avec un client d'API, DumbMonit
redemande un jeton à la minute suivante.

« HPE GreenLake refused the API client » : le client ID ou le secret est
faux. Le secret ne peut pas être réaffiché ; réinitialisez le client dans HPE
GreenLake.

« Aruba Central refused … (403) » : le client n'a pas accès à la surveillance
de Central, ou l'URL de base n'est pas celle de votre région. Une page
facultative refusée ainsi est simplement ignorée.

« did not answer like Aruba Central (no items) » : l'adresse n'est pas une
passerelle d'API Central.

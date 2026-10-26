# Microsoft Azure

La santé d'un abonnement Azure : les ressources qu'Azure signale comme
indisponibles ou dégradées, et les alertes Azure Monitor déclenchées et non
fermées.

DumbMonit lit deux listes sur `management.azure.com`, une fois par minute :
**Resource Health** (`Microsoft.ResourceHealth/availabilityStatuses`, API
2022-10-01), la disponibilité qu'annonce la plate-forme Azure pour chaque
ressource, et les **alertes Azure Monitor**
(`Microsoft.AlertsManagement/alerts`, API 2019-05-05-preview) déclenchées,
nouvelles ou acquittées. L'identifiant est un principal de service : une
application Entra avec un secret client et le rôle **Reader** (Lecteur) sur
l'abonnement. DumbMonit échange le secret contre un jeton d'une heure (flux
« client credentials », portée `https://management.azure.com/.default`) et le
garde jusqu'à expiration. Le rôle Reader ne peut rien modifier.

Une machine virtuelle arrêtée (désallouée) est signalée par Azure comme
indisponible à la demande de son propriétaire : DumbMonit la compte à part et
n'alerte pas.

C'est une API Internet : l'adresse est celle de l'API, pas celle d'une
machine. DumbMonit a besoin de HTTPS sortant vers `management.azure.com` et
`login.microsoftonline.com` ; rien dans vos ressources n'est contacté.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la documentation de l'API
    REST de Microsoft, pas contre un abonnement en fonctionnement. Elle ne
    fait que lire. Dites-nous ce qui casse.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | API Azure Resource Manager (HTTPS, management.azure.com) |
| 443 | TCP | Connexion Microsoft, jeton d'accès (HTTPS, login.microsoftonline.com) |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_azure_`.

| Métrique | Labels | Signification |
|---|---|---|
| `resources` | | Ressources ayant un état Resource Health. |
| `resources_available`, `resources_degraded`, `resources_unavailable`, `resources_stopped`, `resources_unknown` | | Ressources par état ; `stopped` = indisponible à la demande du propriétaire. |
| `resource_unhealthy` | `resource`, `type`, `resource_group`, `state` | Valeur 1 pour chaque ressource dégradée ou indisponible. |
| `alerts_readable` | | 0 quand les alertes ne peuvent pas être lues (voir Dépannage) ; Resource Health reste rapporté. |
| `alerts_active` | `severity` (`Sev0` à `Sev4`) | Alertes déclenchées, nouvelles ou acquittées, par sévérité. |
| `alerts_high_active` | | Alertes déclenchées de sévérité Sev0 ou Sev1. |
| `alerts_total` | | Toutes les alertes déclenchées. |

Au plus 200 ressources nommées et 200 alertes nommées sont listées ; les
compteurs restent exacts. Les listes sont lues sur 20 pages au plus.

Les [règles intégrées](../alerting/rules.md#microsoft-azure) qui s'appliquent :

- **Ressource Azure indisponible** : une ressource indisponible depuis
  10 minutes (Critique).
- **Ressource Azure dégradée** : une ressource dégradée depuis 15 minutes
  (Avertissement).
- **Alerte Azure Monitor Sev0 ou Sev1** : une alerte Sev0 ou Sev1 déclenchée
  et non fermée depuis 5 minutes (Avertissement).

## La page de la surveillance

Le panneau Azure dit ce qui ne va pas en une phrase, montre le nombre de
ressources, les ressources indisponibles et dégradées et les alertes actives,
puis liste les ressources en défaut (et les arrêtées) et les alertes actives.

## Créer une application en lecture seule pour DumbMonit

1. Dans le portail Azure, ouvrez Microsoft Entra ID > App registrations > New registration, nommez-la DumbMonit, gardez un seul locataire, puis Register.

2. Sur la page Overview, copiez l'Application (client) ID et le Directory (tenant) ID.

3. Ouvrez Certificates & secrets > New client secret, choisissez une durée, puis copiez la Value du secret : elle n'est affichée qu'une fois.

4. Ouvrez Subscriptions > votre abonnement > Access control (IAM) > Add > Add role assignment, choisissez le rôle Reader, sélectionnez l'application DumbMonit, puis Review + assign.

5. Copiez le Subscription ID depuis la page Overview de l'abonnement.

6. Dans DumbMonit, saisissez management.azure.com comme adresse, collez l'ID client et le secret, et renseignez les options Tenant ID et Subscription ID.

### Droits requis

- **Reader (rôle sur l'abonnement)** — Lit Resource Health et les alertes Azure Monitor actives ; ne change rien.

!!! warning
    Une machine virtuelle arrêtée ou désallouée est signalée par Azure comme indisponible à la demande de l'utilisateur : elle est comptée à part et ne déclenche aucune alerte. Les alertes Azure Monitor exigent que le fournisseur de ressources Microsoft.AlertsManagement soit enregistré sur l'abonnement ; sans lui, seul Resource Health est surveillé.

Documentation de l'éditeur : https://learn.microsoft.com/rest/api/resourcehealth/availability-statuses/list-by-subscription-id et https://learn.microsoft.com/rest/api/monitor/alertsmanagement/alerts

## Identifiants

| Identifiant | Champs |
|---|---|
| Application Entra (principal de service) | Application (client) ID et Value du secret client. |

Adresse : `management.azure.com`. Options :

- **Tenant ID** (requis) : le Directory (tenant) ID de l'application.
- **Subscription ID** (requis) : l'abonnement à surveiller.
- **URL de connexion** : `https://login.microsoftonline.com` par défaut ;
  ne la changez que pour un cloud souverain.
- **Délai par requête** (15 s).

Le secret client expire (24 mois au plus) : renouvelez-le avant, ou
surveillez-le avec l'intégration Microsoft Entra ID.

## Dépannage

« Microsoft refused the application » : l'ID client ou le secret est
incorrect, ou le secret a expiré.

« Azure refused to read the Resource Health (403) » : l'application n'a pas
le rôle Reader sur cet abonnement ; une attribution de rôle peut prendre
quelques minutes à s'appliquer.

« Azure knows no subscription … » : vérifiez le Subscription ID et le
Tenant ID.

`alerts_readable` vaut 0 : l'application ne peut pas lire les alertes Azure
Monitor, ou le fournisseur de ressources Microsoft.AlertsManagement n'est pas
enregistré sur l'abonnement (Subscriptions > Resource providers).

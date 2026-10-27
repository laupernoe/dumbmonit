# Microsoft Entra ID

Microsoft Entra ID via Microsoft Graph : secrets et certificats d'application
bientôt expirés, synchronisation avec l'Active Directory local en panne, et
administrateurs sans authentification multifacteur.

Un secret d'application expiré est la panne silencieuse d'un locataire Entra :
la sauvegarde, la connexion d'un intranet ou le relais de messagerie qui s'en
servait cesse de fonctionner ce jour-là, sans rien pour vous prévenir.
DumbMonit lit la date d'expiration de chaque secret et de chaque certificat de
chaque inscription d'application (jamais leurs valeurs) et alerte 30 jours
avant.

DumbMonit se connecte comme une application Entra (flux « client
credentials ») : il échange l'ID client et le secret de l'application contre
un jeton d'une heure sur `login.microsoftonline.com`, le garde jusqu'à son
expiration, et lit `graph.microsoft.com` une fois par minute. Chaque appel
est une lecture.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la documentation de
    Microsoft Graph v1.0, pas contre un locataire en fonctionnement. Elle ne
    fait que lire. Dites-nous ce qui casse.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | API Microsoft Graph (HTTPS, graph.microsoft.com) |
| 443 | TCP | Connexion Microsoft, demande de jeton (HTTPS, login.microsoftonline.com) |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_entra_`.

| Métrique | Labels | Signification |
|---|---|---|
| `credential_expiry_seconds` | `app`, `kind` (`secret` ou `certificate`), `key` | Secondes avant l'expiration du secret ou du certificat ; négatif une fois expiré. Les secrets expirés depuis plus de 30 jours ne sont pas publiés. |
| `applications`, `credentials` | | Inscriptions d'application lues, et secrets et certificats trouvés. |
| `credentials_expired` | | Tous les secrets ou certificats expirés, quelle que soit leur ancienneté. |
| `credentials_expiring_30d` | | Secrets et certificats qui expirent dans les 30 jours. |
| `ad_sync_enabled` | | 1 quand le locataire synchronise un annuaire local (Entra Connect). |
| `ad_sync_age_seconds` | | Temps depuis la dernière synchronisation ; seulement quand la synchronisation est activée. |
| `admins`, `admin_without_mfa` | | Administrateurs, et ceux sans méthode MFA enregistrée. |
| `admin_mfa_registered` | `admin` | 0, pour chaque administrateur sans MFA (nom d'utilisateur principal). |
| `mfa_readable` | | 1 quand le rapport d'enregistrement a pu être lu, 0 quand Microsoft l'a refusé. |

Le label `key` est le nom donné au secret ou au certificat, suivi du début de
son identifiant (`prod (aaaa1111)`), pour que deux secrets de même nom restent
distincts.

Seules les applications et l'organisation sont obligatoires. Le rapport MFA
demande une licence Entra ID P1 ou P2 : sans elle, `mfa_readable` vaut 0 et la
sonde réussit quand même.

Les [règles intégrées](../alerting/rules.md#microsoft-entra-id) qui s'appliquent :

- **Secret d'application Entra bientôt expiré** : un secret ou un certificat
  expire dans moins de 30 jours (Avis, rappelé chaque jour).
- **Secret d'application Entra expiré** : un secret ou un certificat a expiré
  ces 30 derniers jours (Avis, rappelé chaque jour). Supprimez-le, ou
  renouvelez-le si son application en a encore besoin.
- **Synchronisation d'annuaire Entra en retard** : la dernière synchronisation
  avec l'Active Directory local date de plus de 3 heures (Avis, rappelé toutes
  les 6 heures).
- **Administrateur Entra sans MFA** : un administrateur n'a aucune méthode
  MFA enregistrée (Avis, rappelé chaque jour).

## La page de la surveillance

Le panneau Entra ID dit ce qui ne va pas en une phrase, montre les secrets qui
expirent ou ont expiré, l'état de la synchronisation d'annuaire et les
administrateurs sans MFA, puis liste les secrets qui expirent dans les 90
jours, l'état de la synchronisation et chaque administrateur sans MFA.

## Créer une application Entra en lecture seule pour DumbMonit

1. Dans le centre d'administration Microsoft Entra (entra.microsoft.com), ouvrez Entra ID → App registrations → New registration, nommez-la DumbMonit, gardez un seul locataire et enregistrez-la.

2. Sur la page Overview, copiez l'Application (client) ID et le Directory (tenant) ID.

3. Ouvrez API permissions → Add a permission → Microsoft Graph → Application permissions, ajoutez les droits listés ci-dessous, puis cliquez sur Grant admin consent.

4. Ouvrez Certificates & secrets → New client secret et copiez sa Value (pas le Secret ID) : elle n'est affichée qu'une fois et le secret expire, notez donc la date.

5. Dans DumbMonit, saisissez graph.microsoft.com comme adresse, collez l'ID client et la valeur du secret, et renseignez l'option Tenant ID.

### Droits requis

Ajoutez-les comme permissions d'application (pas déléguées), puis accordez le consentement administrateur.

- **Application.Read.All** — Lit les inscriptions d'application : la date d'expiration de chaque secret et certificat, jamais leurs valeurs.
- **Organization.Read.All** — Lit le locataire et l'état de sa synchronisation d'annuaire local.
- **Reports.Read.All** — Lit l'enregistrement MFA des administrateurs ; demande Entra ID P1 ou P2 (facultatif).

!!! warning
    Le consentement administrateur donne à l'application un accès en lecture à tout le locataire pour les données ci-dessus. Utilisez une inscription d'application dédiée avec ces seuls droits, et renouvelez son secret client avant son expiration : DumbMonit surveille son propre secret comme les autres.

Documentation de l'éditeur : https://learn.microsoft.com/en-us/graph/api/application-list

### Notes

La même inscription d'application peut servir à l'intégration Microsoft 365 :
ajoutez-lui les droits des deux pages et utilisez les mêmes identifiants.
DumbMonit surveille son propre secret client comme les autres : la règle
**Secret d'application Entra bientôt expiré** se déclenche 30 jours avant qu'il
cesse de fonctionner.

## Identifiants

| Identifiant | Champs |
|---|---|
| Application Entra | Application (client) ID et Value du secret client. |

Adresse : `graph.microsoft.com`. Options :

- **Tenant ID** (obligatoire) : le Directory (tenant) ID, un GUID, ou un
  nom de domaine vérifié.
- **Sign-in URL** : `https://login.microsoftonline.com` par défaut. Ne le
  changez que pour un cloud souverain (par exemple
  `https://login.microsoftonline.us`, avec `graph.microsoft.us` comme
  adresse).
- **Délai par requête (secondes)** (15 s).

## Dépannage

« Microsoft refused the application » : l'ID client ou le secret est
incorrect, ou le secret a expiré. Copiez la Value du secret, pas son ID.

« Microsoft Graph refused … (403): grant the application the
Application.Read.All application permission » (ou `Organization.Read.All`) :
ajoutez ce droit et cliquez sur Grant admin consent ; l'application peut
prendre quelques minutes.

`mfa_readable` reste à 0 : le locataire n'a pas de licence Entra ID P1 ou P2,
ou `Reports.Read.All` n'a pas été accordé. Rien n'est cassé.

Pas de `ad_sync_age_seconds` : le locataire ne synchronise pas d'annuaire
local (`ad_sync_enabled` vaut 0), ou Microsoft n'indique pas encore de date de
synchronisation.

Un secret supprimé du portail reste listé une minute : attendez la prochaine
sonde.

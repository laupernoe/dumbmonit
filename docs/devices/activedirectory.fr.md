# Active Directory

Un domaine Windows via LDAP, en lecture seule : les contrôleurs de domaine
et qui détient chaque rôle FSMO, les membres des groupes privilégiés, la
politique de mot de passe, les compteurs de comptes, et les faits de
sécurité bruts qu'un audit cherche en premier — comptes exposés au
Kerberoasting ou à l'AS-REP roasting, délégation sans contrainte, un vieux
mot de passe krbtgt, ordinateurs sans LAPS, versions de Windows hors
support.

DumbMonit se connecte avec un compte de service dédié qui est un
**utilisateur de domaine ordinaire, membre d'aucun groupe**. Dans Active
Directory, chaque utilisateur authentifié peut lire ce que DumbMonit lit ;
aucun droit administratif n'est nécessaire, et aucun ne devrait être
donné. DumbMonit n'écrit jamais dans l'annuaire, ne lit jamais un
attribut de mot de passe (de LAPS il lit la date d'expiration, jamais le
mot de passe) et ne demande jamais les droits de réplication qui
exposeraient des secrets.

Un seul appareil couvre tout le domaine : pointez-le vers un contrôleur
de domaine et DumbMonit trouve les autres dans la partition de
configuration.

## Ce que LDAP peut et ne peut pas dire

LDAP montre l'annuaire comme il est, pas les journaux de ce qui s'y est
passé. En particulier :

- **La réplication.** Le contrôleur de domaine interrogé ne publie
  l'état de ses partenaires de réplication entrants (dernier succès,
  échecs consécutifs, dernière erreur) qu'à un compte détenant le droit
  *Monitor active directory replication*. Sans lui, le panneau dit
  *statut de réplication non lisible* plutôt que de deviner. Seule la
  vue du contrôleur de domaine interrogé est montrée : pour voir les
  partenaires d'un autre, ajoutez-le comme un second appareil.
- **Pas visible via LDAP** : journaux d'événements (connexions,
  verrouillages au moment où ils arrivent, changements de groupe comme
  événements), contenu de la Group Policy, SYSVOL, zones DNS, et les
  hashs de mot de passe eux-mêmes.
- **Les dates de dernière connexion** viennent de
  `lastLogonTimestamp`, que les contrôleurs de domaine ne répliquent que
  tous les 9 à 14 jours : un compte compté comme inactif est inactif à
  deux semaines près.
- **L'appartenance aux groupes** est lue avec imbrication (la règle
  `LDAP_MATCHING_RULE_IN_CHAIN`), via `member`/`memberOf`. Un groupe
  principal réglé par `primaryGroupID` n'est pas un lien `member` et
  n'est pas vu.

## Comment il lit

Deux rythmes :

- **À chaque vérification** (chaque minute par défaut) : connexion et
  bind, le RootDSE (niveaux fonctionnels, horloge, synchronisation),
  l'objet domaine (politique de mot de passe, quota de comptes machine),
  les cinq rôles FSMO, les contrôleurs de domaine depuis la partition de
  configuration, les groupes privilégiés et leurs membres effectifs, la
  fonctionnalité Corbeille, et une connexion TCP vers le port LDAP de
  chaque autre contrôleur de domaine.
- **En arrière-plan** toutes les `inventory_minutes` (15 par défaut) :
  chaque compte utilisateur et ordinateur, avec seulement les attributs
  dont les constats ont besoin, par pages de 500. Énumérer tout
  l'annuaire chaque minute chargerait les contrôleurs de domaine pour
  rien.

La première vérification attend quelques secondes l'inventaire ; sur un
grand domaine, les constats sur les comptes apparaissent à la vérification
suivante.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_ad_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `bind_ok` | 1 quand le compte de service est accepté, 0 quand le contrôleur de domaine le refuse | |
| `connect_seconds`, `bind_seconds` | temps pour ouvrir la connexion (TLS inclus) et pour se connecter | |
| `tls_verified` | 1 quand le certificat est vérifié | |
| `domain_info` | valeur 1 | `domain`, `forest`, `dc` |
| `domain_functional_level`, `forest_functional_level` | 0 (2000) à 7 (2016), 10 (2025) | |
| `dc_synchronized`, `dc_global_catalog_ready` | le contrôleur de domaine interrogé a terminé sa réplication initiale, son catalogue global est prêt | |
| `clock_skew_seconds` | horloge du contrôleur de domaine moins horloge de l'hôte DumbMonit | |
| `recycle_bin_enabled` | 1 quand la Corbeille AD est activée | |
| `dc_count` | contrôleurs de domaine du domaine | |
| `dc_info` | valeur 1 | `dc`, `site`, `gc`, `rodc`, `os` |
| `dc_reachable` | 1 ou 0 par contrôleur de domaine ; aucune série quand son nom ne se résout pas depuis DumbMonit | `dc` |
| `fsmo_role` | valeur 1 | `role` (`schema`, `domain_naming`, `pdc`, `rid`, `infrastructure`), `holder` |
| `replication_readable` | 1 quand le statut de réplication entrante peut être lu | |
| `replication_consecutive_failures`, `replication_last_success_age_seconds` | par partenaire entrant et partition | `source`, `naming_context` |
| `privileged_group_members`, `privileged_group_enabled_members` | membres effectifs, imbrication incluse, groupes laissés de côté | `group` |
| `privileged_group_fingerprint` | un hash des membres : change quand quelqu'un rejoint ou part | `group` |
| `users_total`, `users_enabled`, `users_disabled`, `users_locked_out`, `users_password_never_expires`, `users_stale` | comptes utilisateur | |
| `computers_total`, `computers_enabled`, `computers_stale`, `groups_total` | ordinateurs et groupes | |
| `admincount_accounts` | utilisateurs protégés par AdminSDHolder (`adminCount=1`) | |
| `krbtgt_password_age_seconds` | âge du mot de passe krbtgt | |
| `laps_schema`, `laps_computers`, `laps_eligible_computers` | LAPS dans le schéma ; ordinateurs non-DC activés avec une expiration LAPS, sur tous | |
| `password_min_length`, `password_history_length`, `password_max_age_seconds` (0 = jamais), `password_complexity`, `lockout_threshold`, `machine_account_quota` | politique de domaine par défaut | |
| `finding_count` | objets concernés par un constat, 0 quand examiné et sain | `finding`, `severity`, `title` |
| `inventory_age_seconds`, `scrape_errors` | âge du dernier inventaire complet ; parties de l'annuaire qui n'ont pas pu être lues | |

Les groupes privilégiés sont trouvés par leur identifiant bien connu, pas
leur nom, un domaine installé dans une autre langue fonctionne donc
pareil : Domain Admins (RID 512), Enterprise Admins (519) et Schema
Admins (518) — les deux derniers seulement dans le domaine racine de la
forêt — et les intégrés Administrators, Account Operators, Server
Operators, Print Operators et Backup Operators.

## Constats

Les constats sont des faits bruts — combien d'objets, et lesquels — pas
un score ; le [score de sécurité](../using/security-score.md) de
l'appareil les pèse. Chacun a un `id` stable, une gravité sur l'échelle
du score (low, medium, high, critical, montrée sur la page comme Info,
Avis et Avertissement), une catégorie, un compteur et jusqu'à vingt noms
de compte en exemple. Un constat qui a été examiné et n'a rien trouvé est
gardé avec un compteur de 0, un problème corrigé devient donc vert plutôt
que de disparaître ; les constats sur les comptes apparaissent une fois
le premier inventaire en arrière-plan terminé. Ils sont servis tels
quels par `GET /api/targets/{id}/ad/findings` et publiés comme
`dumbmonit_ad_finding_count{finding, severity, title}`.

| `id` | Gravité | Quoi |
|---|---|---|
| `unconstrained_delegation` | critical | comptes activés de confiance pour une délégation sans contrainte, contrôleurs de domaine à part |
| `kerberoastable_privileged` | critical | membres activés de groupes privilégiés avec un nom principal de service |
| `asrep_roastable_users` | high | comptes activés avec la préauthentification Kerberos désactivée (`DONT_REQ_PREAUTH`) |
| `kerberoastable_users` | high | utilisateurs activés avec un nom principal de service |
| `krbtgt_password_age` | high | mot de passe krbtgt plus ancien que 180 jours |
| `laps_coverage` | high quand LAPS est absent, medium quand partiel | ordinateurs activés (contrôleurs de domaine à part) sans mot de passe LAPS |
| `reversible_encryption` | high | utilisateurs dont le mot de passe est stocké avec chiffrement réversible |
| `guest_enabled` | high | le compte Guest intégré (RID 501) est activé |
| `obsolete_os` | high | ordinateurs activés sous Windows XP, Vista, 7, 8, 2000, Server 2003, 2008 ou 2012 |
| `dc_unreachable` | high | contrôleurs de domaine dont le port LDAP ne répond pas |
| `replication_failing` | high | partenaires de réplication entrante avec des échecs consécutifs |
| `dc_not_synchronized` | high | le contrôleur de domaine interrogé n'a pas terminé sa réplication initiale |
| `fsmo_role_orphaned` | high | un rôle FSMO détenu par un contrôleur de domaine supprimé |
| `password_policy` | high sous 8 caractères ou sans verrouillage, medium sinon | lacunes dans la politique de domaine par défaut contre la référence : 14 caractères, un seuil de verrouillage, complexité |
| `privileged_group_size` | medium | plus de 5 comptes activés dans Domain Admins, Enterprise Admins et Administrators ensemble |
| `stale_accounts` | medium | utilisateurs et ordinateurs activés inactifs depuis plus que les seuils |
| `protocol_transition_delegation` | medium | délégation contrainte avec transition de protocole |
| `des_only` | medium | comptes restreints aux clés Kerberos DES |
| `password_not_required` | medium | utilisateurs autorisés à avoir un mot de passe vide (`PASSWD_NOTREQD`) |
| `privileged_password_never_expires` | medium | utilisateurs privilégiés dont le mot de passe n'expire jamais |
| `privileged_stale` | medium | utilisateurs privilégiés inactifs depuis plus que `stale_days_users` |
| `schema_admins_not_empty` | medium | Schema Admins a des membres |
| `functional_level_old` | medium | niveau fonctionnel de domaine sous 2012 R2 |
| `clock_skew` | medium | plus de cinq minutes entre le contrôleur de domaine et DumbMonit |
| `windows10_end_of_support` | medium | ordinateurs activés sous Windows 10 |
| `admincount_orphans` | low | utilisateurs `adminCount=1` plus dans aucun groupe privilégié |
| `password_never_expires` | low | utilisateurs activés dont le mot de passe n'expire jamais |
| `machine_account_quota` | low | n'importe quel utilisateur peut joindre des ordinateurs au domaine |
| `recycle_bin_disabled` | low | la Corbeille AD n'est pas activée |

Les [règles intégrées](../alerting/rules.md#active-directory) qui
s'appliquent :

- **Contrôleur de domaine inaccessible** : un autre contrôleur de
  domaine du domaine arrête de répondre sur son port LDAP depuis cinq
  minutes (Avertissement). Celui interrogé est couvert par *Appareil
  inaccessible*.
- **Connexion Active Directory échouée** : le contrôleur de domaine
  refuse le compte de service — mot de passe expiré, désactivé ou
  verrouillé (Avis).
- **Appartenance à un groupe privilégié changée** : quelqu'un a rejoint
  ou quitté un groupe privilégié, imbrication incluse (Avis, se résout
  une heure plus tard).
- **Réplication Active Directory en échec** : un partenaire entrant
  échoue pendant une demi-heure (Avis), quand le statut de réplication
  est lisible.
- **Mot de passe krbtgt plus ancien que 180 jours** (Info).

## La page de l'appareil

Le panneau Active Directory montre le domaine (niveaux fonctionnels, le
contrôleur de domaine interrogé, décalage d'horloge), les contrôleurs de
domaine avec leur site, rôles et s'ils répondent, les groupes privilégiés
avec leurs membres, la politique de mot de passe et les compteurs de
compte, et le tableau des constats avec les mots de gravité.

## Créer un compte de service en lecture seule pour DumbMonit

1. Créez un utilisateur dédié pour DumbMonit, par exemple svc-dumbmonit, et ne l'ajoutez à aucun groupe. Un utilisateur de domaine ordinaire suffit : chaque utilisateur authentifié peut lire ce que DumbMonit lit. En PowerShell, avec le module Active Directory :

    ```
    New-ADUser -Name svc-dumbmonit -UserPrincipalName svc-dumbmonit@corp.example.com -AccountPassword (Read-Host -AsSecureString 'Password') -Enabled $true
    ```

2. Donnez-lui un mot de passe long et aléatoire. Si les mots de passe expirent dans votre domaine, DumbMonit déclenche Connexion Active Directory échouée le jour où celui-ci le fait.

3. Optionnel : pour voir le statut de réplication entrante, accordez au compte le droit Monitor active directory replication sur l'objet domaine lui-même. Il ne révèle aucun mot de passe ni secret.

    ```
    dsacls "DC=corp,DC=example,DC=com" /G "CORP\svc-dumbmonit:CA;Monitor active directory replication"
    ```

4. LDAPS a besoin d'un certificat sur chaque contrôleur de domaine ; une autorité de certification d'entreprise (AD CS) en émet un automatiquement. Exportez le certificat de cette autorité au format X.509 encodé en Base-64 et collez-le comme Certificat CA ci-dessous.

5. Dans DumbMonit, saisissez le nom d'un contrôleur de domaine comme son certificat le nomme, par exemple « dc1.corp.example.com », et connectez-vous comme svc-dumbmonit@corp.example.com. DumbMonit trouve les autres contrôleurs de domaine par lui-même et vérifie que chacun répond.

6. DumbMonit ne fait que lire : il n'écrit jamais dans l'annuaire, et de LAPS il lit la date d'expiration, jamais le mot de passe.

!!! warning
    Le LDAP simple sur le port 389 envoie le mot de passe du compte de service en texte clair : DumbMonit le refuse sauf si vous cochez Autoriser le LDAP simple, et un contrôleur de domaine qui exige la signature LDAP le refuse aussi. Utilisez LDAPS, ou StartTLS sur le port 389.

Pour empêcher le compte d'ouvrir une session n'importe où, refusez-lui
*Log on locally* et *Log on through Remote Desktop Services* par Group
Policy : un bind LDAP n'est pas affecté. Ne le restreignez pas avec *Log
On To* ou *Deny access to this computer from the network* : les
contrôleurs de domaine refuseraient alors le bind.

### Moindre privilège, en détail

- Le compte n'a besoin d'aucun groupe au-delà de Domain Users, que chaque
  compte a.
- N'utilisez pas un compte de service managé (gMSA) : il ne peut pas
  faire un simple bind LDAP.
- N'accordez pas *Replicating Directory Changes* (ou *All*) : c'est le
  droit DCSync, qui expose chaque hash de mot de passe. DumbMonit n'en a
  jamais besoin.
- *Monitor active directory replication* ne permet que de lire les
  métadonnées de réplication (partenaires, dernier succès, codes
  d'erreur). C'est optionnel.

## LDAPS et certificats

- **LDAPS (port 636)** est le défaut. Chaque contrôleur de domaine d'un
  domaine avec une autorité de certification d'entreprise reçoit un
  certificat automatiquement ; sans ça, le contrôleur de domaine
  n'écoute pas sur 636. Le certificat doit porter le nom que DumbMonit
  utilise : saisissez le contrôleur de domaine par son nom DNS, pas son
  adresse IP, sauf si le certificat liste l'adresse.
- **Certificat CA** : collez le PEM de l'autorité racine (ou
  émettrice). Seule cette autorité est alors de confiance. Sans ça,
  DumbMonit fait confiance aux racines publiques et au magasin système de
  son conteneur, qui ne connaissent pas une autorité d'entreprise
  privée.
- **StartTLS (port 389)** chiffre de la même façon, après une
  salutation en texte clair. Utilisez-le quand le port 636 est filtré.
- **Accepter un certificat non vérifiable** garde la connexion chiffrée
  mais accepte n'importe quel certificat : quiconque sur le chemin
  pourrait se faire passer pour le contrôleur de domaine et recevoir le
  mot de passe du compte de service.
- **La liaison de canal LDAP** (`LdapEnforceChannelBinding`) concerne
  les binds NTLM et Kerberos sur TLS ; le bind simple que DumbMonit
  utilise n'est pas affecté.

## Identifiants

| Champ | Valeur |
|---|---|
| Nom d'utilisateur | `svc-dumbmonit@corp.example.com`, `CORP\svc-dumbmonit`, ou le DN complet du compte |
| Mot de passe | le mot de passe du compte, stocké chiffré et jamais remontré |

Un mot de passe vide est refusé : Active Directory l'accepterait comme
un bind non authentifié sans rien vérifier.

## Options

| Option | Défaut | Quoi |
|---|---|---|
| Sécurité de connexion (`security`) | `ldaps` | `ldaps`, `starttls` ou `plain` |
| Port (`port`) | 636, ou 389 | utilisé si l'adresse n'en donne pas |
| Certificat CA (`ca_cert`) | | PEM de l'autorité qui a émis les certificats des contrôleurs de domaine |
| Accepter un certificat non vérifiable (`insecure_tls`) | désactivé | |
| Autoriser le LDAP simple (`allow_plaintext`) | désactivé | requis pour `plain` |
| Vérifier chaque contrôleur de domaine (`check_all_dcs`) | activé | connexion TCP au port LDAP de chaque contrôleur de domaine |
| Utilisateur inactif après (`stale_days_users`) | 90 | jours |
| Ordinateur inactif après (`stale_days_computers`) | 90 | jours |
| Inventaire complet toutes les (`inventory_minutes`) | 15 | minutes, de 1 à 1440 |
| Délai (`request_timeout_seconds`) | 8 | par opération LDAP, de 1 à 60 |

## Dépannage

- **Certificat TLS rejeté** : collez le certificat de l'autorité comme
  Certificat CA, et saisissez le contrôleur de domaine par un nom que
  son certificat liste.
- **Bind refusé : mauvais nom d'utilisateur ou mot de passe (LDAP result
  49)** : le code de diagnostic du contrôleur de domaine est traduit —
  mot de passe expiré, compte désactivé, verrouillé, ou doit changer son
  mot de passe à la prochaine connexion.
- **Le contrôleur de domaine exige une connexion protégée** : la
  signature LDAP est imposée ; passez à LDAPS ou StartTLS.
- **Un contrôleur de domaine se montre non résolu** : son nom ne se
  résout pas depuis l'hôte DumbMonit. Pointez le DNS de DumbMonit vers
  les serveurs DNS du domaine, ou décochez *Vérifier chaque contrôleur
  de domaine*. Les noms non résolus ne déclenchent jamais d'alerte.
- **Statut de réplication non lisible** : accordez *Monitor active
  directory replication* comme à l'étape 3, ou acceptez que la
  réplication ne soit pas montrée.

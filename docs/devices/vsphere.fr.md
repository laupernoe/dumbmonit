# VMware vSphere

Un vCenter Server et les hôtes qu'il gère, ou un hôte ESXi autonome : hôtes
qui arrêtent de répondre, sont laissés en mode maintenance ou rapportent une
santé rouge, machines virtuelles et leurs VMware Tools, datastores qui se
remplissent ou deviennent inaccessibles, et les alarmes que vSphere lui-même
a levées.

DumbMonit lit vSphere avec un utilisateur dédié portant le rôle intégré
**Read-only**. Il n'allume ni n'éteint jamais une VM, n'entre jamais en mode
maintenance et n'acquitte jamais une alarme.

La panne à surveiller : **un datastore qui se remplit**. Quand un datastore
en provisionnement fin atteint 100 %, chaque VM qui a besoin d'y écrire est
mise en pause d'un coup. Les alarmes que vCenter lève le disent aussi, mais
seulement à qui a le client ouvert.

## Pourquoi la Web Services API

DumbMonit parle à la Web Services API de vSphere (SOAP, `/sdk`), pas à l'API
REST (`/api/vcenter/*`) : un hôte ESXi autonome n'a pas d'API REST, et sur un
vCenter l'API REST ne dit pas le mode maintenance, la santé globale
(`overallStatus`), l'état des VMware Tools ou les alarmes déclenchées. Une vue
de tout l'inventaire est lue en une seule passe, page par page, et libérée à
la fin.

La session est gardée d'une sonde à l'autre et rouverte seulement quand elle
expire : vCenter n'enregistre pas une nouvelle connexion chaque minute.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_vsphere_`. Les valeurs de
santé suivent le `overallStatus` de vSphere : 0 vert, 1 jaune, 2 rouge, 3 gris
(inconnu).

| Métrique | Quoi | Labels |
|---|---|---|
| `info` | valeur 1 | `product` (`vcenter` ou `esxi`), `version`, `build` |
| `hosts` | hôtes par état de connexion : `connected`, `disconnected`, `notResponding` | `state` |
| `host_connection_state` | 0 connecté, 1 déconnecté par un administrateur, 2 ne répond pas | `host` |
| `host_power_state` | 0 allumé, 1 veille, 2 éteint, 3 inconnu | `host` |
| `host_maintenance` | 1 en mode maintenance | `host` |
| `host_status` | santé globale | `host` |
| `host_cpu_percent`, `host_memory_percent`, `host_memory_bytes`, `host_uptime_seconds` | hôtes connectés seulement | `host` |
| `vms` | VM par état d'alimentation : `poweredOn`, `poweredOff`, `suspended` (templates exclus) | `state` |
| `templates` | templates de VM | |
| `vm_power_state` | 0 allumée, 1 éteinte, 2 suspendue ; 1000 VM au plus | `vm` |
| `vm_status` | santé globale | `vm` |
| `vm_tools_status` | VM allumées seulement : 0 à jour, 1 vieille version, 2 installé mais pas lancé, 3 pas installé | `vm` |
| `datastores` | datastores | |
| `datastore_capacity_bytes`, `datastore_free_bytes`, `datastore_used_percent` | | `datastore` |
| `datastore_accessible` | 1 accessible, 0 inaccessible | `datastore` |
| `datastore_status` | santé globale | `datastore` |
| `alarms` | alarmes déclenchées non acquittées, par couleur : `red`, `yellow` | `status` |
| `alarms_acknowledged` | alarmes déclenchées déjà acquittées | |
| `alarm` | une série par alarme non acquittée, valeur = son code de santé, 64 au plus | `alarm` (son nom), `entity`, `entity_type`, `status` |
| `scrape_errors` | appels qui ont échoué à la dernière sonde | |

Les [règles intégrées](../alerting/rules.md#vmware-vsphere) qui
s'appliquent : hôte vSphere ne répond pas, santé d'hôte vSphere rouge, hôte
vSphere laissé en maintenance, datastore vSphere presque plein, datastore
vSphere inaccessible, VMware Tools vSphere non lancés, alarme rouge vSphere,
plus Surveillance inaccessible.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais vSphere. Il dit d'abord ce qui ne va pas (hôtes qui
ne répondent pas, alarmes rouges, datastores presque pleins ou
inaccessibles), puis les hôtes avec leur charge, les datastores avec leur
niveau de remplissage, et les VM par état d'alimentation.

## Comment cela a été validé

Contre le simulateur govmomi `vcsim` 0.56.0, en mode vCenter et en mode ESXi :
connexion et mot de passe refusé, réutilisation et expiration de session,
pagination dans l'inventaire, un hôte qui ne répond pas, un hôte en mode
maintenance, une VM éteinte et un template. Le simulateur ne lève aucune
alarme : lire les alarmes déclenchées est validé contre la structure
`AlarmState` documentée de la Web Services API.

## Créer un utilisateur en lecture seule dans vSphere

1. Sur un vCenter : Administration → Single Sign On → Users and Groups → Users, domaine vsphere.local → Add. Nommez l'utilisateur comme suit et donnez-lui un mot de passe long et aléatoire.

    ```
    dumbmonit
    ```

2. Toujours sur le vCenter : Administration → Access Control → Global Permissions → Add. Choisissez cet utilisateur, le rôle Read-only, et cochez Propagate to children. Read-only voit chaque hôte, VM, datastore et alarme, et ne peut rien changer.

3. Sur un hôte ESXi autonome à la place : dans le host client, Manage → Security & users → Users → Add user, avec le même nom. Puis Host → Actions → Permissions → Add user : choisissez-le, le rôle Read-only, et cochez Propagate to all children.

4. Dans DumbMonit, saisissez l'adresse du vCenter ou de l'hôte, par exemple « vcenter.lan » ou « esxi1.lan », avec le nom d'utilisateur tel que créé : dumbmonit@vsphere.local sur un vCenter, dumbmonit sur un hôte. Quand un vCenter gère les hôtes, ajoutez seulement le vCenter : il rapporte chaque hôte, VM et datastore.

5. DumbMonit ne fait que lire, via la Web Services API de vSphere (/sdk). Il n'allume ni n'éteint jamais une VM, n'entre jamais en mode maintenance et n'acquitte jamais une alarme.

!!! warning
    Ne réutilisez pas le compte de domaine du single sign-on ni le compte intégré de l'hôte : le rôle Read-only ci-dessus est tout ce dont DumbMonit a besoin. vCenter et ESXi utilisent un certificat autosigné par défaut : activez « Accepter un certificat non vérifiable » sauf si l'hôte DumbMonit fait confiance à leur autorité de certification.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur / mot de passe | L'utilisateur dédié avec le rôle Read-only : `dumbmonit@vsphere.local` sur un vCenter, `dumbmonit` sur un hôte. |

Adresse : un nom d'hôte ou une IP (`vcenter.lan`), `host:port`, ou une URL
complète ; un `/sdk` ou `/ui` final est accepté. Options : port (443),
vérification du certificat, délai de requête (20 s) et alarmes déclenchées
(lues).

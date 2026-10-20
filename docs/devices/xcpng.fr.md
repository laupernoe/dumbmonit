# XCP-ng / XenServer

Un pool XCP-ng ou XenServer (Citrix Hypervisor), lu via XAPI, l'API de
gestion que chaque hôte exécute déjà. Aucun Xen Orchestra n'est nécessaire.

Chaque minute, DumbMonit envoie des appels JSON-RPC à
`https://<coordinateur>/jsonrpc` : un `session.login_with_password` (la
session est ensuite conservée d'une interrogation à l'autre, et rouverte une
fois si le pool l'invalide), et un `get_all_records` par classe : `host`,
`host_metrics`, `VM`, `VM_metrics`, `SR` et `pool`. Rien n'est jamais
modifié : le compte a le rôle intégré `read-only`, que XAPI applique à chaque
appel.

Ajoutez le pool une seule fois, par l'adresse de son coordinateur : il
rapporte tous les hôtes, VM et référentiels de stockage. Si l'adresse saisie
est celle d'un membre du pool, il répond `HOST_IS_SLAVE` avec l'adresse du
coordinateur ; DumbMonit la suit une fois, même protocole et même port, de
sorte qu'un changement de coordinateur ne vous oblige pas à modifier
la surveillance.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la documentation de XAPI
    (le protocole JSON-RPC, les classes `host`, `VM`, `SR` et `pool`) et
    testée contre des réponses enregistrées, pas contre un pool en
    fonctionnement. Dites-nous ce qui casse.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_xcpng_`.

| Métrique | Labels | Signification |
|---|---|---|
| `hosts`, `hosts_enabled` | | Hôtes du pool, et combien acceptent de nouvelles VM. |
| `host_info` | `host`, `version` | Valeur 1 : la version du produit de chaque hôte. |
| `host_enabled` | `host` | 0 quand l'hôte est désactivé (maintenance, ou évacué). |
| `host_live` | `host` | 1 quand le battement de cœur de l'hôte atteint le coordinateur, 0 quand il est hors ligne. |
| `host_cpu_cores`, `host_memory_total_bytes`, `host_memory_used_bytes` | `host` | Capacité et mémoire utilisée. |
| `vms`, `vms_running` | | VM, sans les modèles, les instantanés ni les domaines de contrôle. |
| `vm_power_state_info` | `vm`, `state` | Valeur 1 : `running`, `halted`, `paused` ou `suspended`. |
| `vm_running` | `vm` | 1 quand la VM tourne. |
| `vm_vcpus`, `vm_memory_bytes` | `vm` | vCPU au démarrage ; mémoire utilisée pour une VM en marche, son maximum configuré sinon. |
| `vm_ha_protected_down` | `vm` | Seulement pour les VM avec une priorité de redémarrage, dans un pool où la haute disponibilité est activée : 1 quand la VM est arrêtée. |
| `srs` | | Référentiels de stockage. |
| `sr_size_bytes`, `sr_used_bytes`, `sr_used_percent` | `sr` | Niveau de remplissage (les bibliothèques ISO sont comptées, pas détaillées). |
| `sr_shared` | `sr` | 1 pour un référentiel partagé (NFS, iSCSI…). |
| `pool_info` | `pool`, `master` | Valeur 1 : le pool et son coordinateur actuel. |
| `pool_ha_enabled` | `pool` | 1 quand la haute disponibilité est activée. |
| `pool_ha_overcommitted` | `pool` | Avec la HA activée : 1 quand le pool ne peut plus redémarrer ses VM protégées après les pannes d'hôtes qu'il est réglé pour tolérer. |
| `pool_ha_host_failures_to_tolerate` | `pool` | Le nombre de pannes d'hôtes que la HA est réglée pour survivre. |

XenAPI n'a pas d'état d'alimentation « planté ». Une VM protégée trouvée
arrêtée est le signal le plus proche que l'API donne d'une VM qui s'est
arrêtée contre votre gré ; si vous éteignez volontairement une VM protégée,
supprimez d'abord sa priorité de redémarrage.

Les [règles intégrées](../alerting/rules.md#xcp-ng-xenserver) qui s'appliquent :

- **Hôte XCP-ng hors ligne** : le battement de cœur d'un hôte n'a pas atteint
  le coordinateur depuis cinq minutes (Avertissement).
- **SR XCP-ng presque plein** : un référentiel de stockage est plein à 90 %
  pendant quinze minutes (Avis).
- **VM XCP-ng protégée par HA arrêtée** : une VM protégée par la haute
  disponibilité est arrêtée depuis deux minutes (Avertissement).
- **HA du pool XCP-ng en surengagement** : le pool est en surengagement
  depuis cinq minutes (Avis).

## Donner à DumbMonit un compte XAPI en lecture seule

1. Sur chaque hôte du pool, créez un compte local avec un long mot de passe aléatoire.

    ```
    useradd -M -s /sbin/nologin dumbmonit
    passwd dumbmonit
    ```

2. Sur le coordinateur du pool, activez une fois l'authentification PAM.

    ```
    xe pool-enable-external-auth auth-type=PAM service-name=pam
    ```

3. Enregistrez le compte comme sujet et notez l'UUID affiché.

    ```
    xe subject-add subject-name=dumbmonit
    ```

4. Donnez à ce sujet le rôle intégré read-only, avec l'UUID.

    ```
    xe subject-role-add uuid=SUBJECT_UUID role-name=read-only
    ```

5. Dans DumbMonit, saisissez l'adresse du coordinateur du pool (par exemple xcp01.lan) avec ce nom d'utilisateur et ce mot de passe.

### Droits requis

- **Rôle read-only (XAPI)** : Voit les hôtes, VM, dépôts de stockage et la HA

!!! warning
    Si le pool utilise déjà Active Directory, créez le compte là-bas et sautez les étapes 1 et 2. XCP-ng utilise un certificat autosigné : activez « Accepter un certificat non vérifiable » sauf si l'hôte de DumbMonit lui fait confiance.

Documentation de l'éditeur : [XenServer RBAC with the CLI](https://docs.xenserver.com/en-us/xenserver/9/users/rbac-cli)

### Pourquoi ces droits

XAPI n'a pas d'écran pour cela : son contrôle d'accès par rôles se gère en
ligne de commande avec `xe`, les commandes ci-dessus sont donc la voie à suivre
(Xen Orchestra gère ses propres utilisateurs, pas les sujets XAPI).

XAPI n'accepte d'autres comptes que son superutilisateur intégré qu'une fois le
pool en authentification externe, d'où l'activation de PAM. Tant qu'aucun rôle
n'est donné, le compte ne peut rien appeler ; avec `read-only`, un mot de passe
divulgué ne peut rien modifier. Ne donnez jamais à DumbMonit le mot de passe
superutilisateur des hôtes.

Ajoutez le pool une seule fois : le coordinateur remonte chaque hôte, VM et
dépôt de stockage. Un membre du pool répond avec l'adresse du coordinateur,
que DumbMonit suit alors.

DumbMonit ne fait que lire, via l'interface JSON-RPC de XAPI (`/jsonrpc`) : il
ne démarre, n'arrête ni ne migre jamais une VM et ne modifie jamais un dépôt
de stockage ni la haute disponibilité du pool.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur et mot de passe | Le compte `dumbmonit`, enregistré comme sujet avec le rôle `read-only`. |

Adresse : le nom d'hôte ou l'IP du coordinateur du pool, `host:port`, ou une
URL. Options :

- **Port** (443).
- **Accepter un certificat non vérifiable** : le propre certificat des hôtes
  est autosigné.
- **Délai par requête** (15 s).

## Dépannage

« XCP-ng refused the user name or password » : mauvais mot de passe, ou le
compte n'est pas enregistré comme sujet (`xe subject-list`).

« XCP-ng accepted the account but refused the call » : le sujet n'a aucun
rôle, ou un rôle sans accès en lecture ; donnez-lui `read-only`.

« This host is a pool member, not the coordinator » : DumbMonit n'a pas pu
joindre le coordinateur que le membre a indiqué ; saisissez vous-même
l'adresse du coordinateur.

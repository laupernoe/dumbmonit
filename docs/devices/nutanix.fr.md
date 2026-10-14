# Nutanix AHV

Des clusters Nutanix, lus via l'API REST v4 de Prism Central. Un seul Prism
Central couvre tous les clusters qui y sont enregistrés.

Chaque minute, DumbMonit lit, avec l'authentification HTTP basique sur le
port 9440 :

| Quoi | Chemin |
|---|---|
| Clusters | `/api/clustermgmt/v4.0/config/clusters` |
| Hôtes | `/api/clustermgmt/v4.0/config/hosts` |
| VM AHV | `/api/vmm/v4.0/ahv/config/vms` |
| Conteneurs de stockage | `/api/clustermgmt/v4.0/config/storage-containers` |
| Niveau de remplissage de chaque conteneur | `/api/clustermgmt/v4.0/stats/storage-containers/{extId}` |
| Alertes non résolues | `/api/monitoring/v4.0/serviceability/alerts` |

Les listes sont lues par pages de 100 éléments, jusqu'à 1 000 éléments chacune.
Le niveau de remplissage et les alertes sont facultatifs : si le rôle du
compte les refuse, le reste est quand même collecté. Nutanix a déclaré
obsolètes les API v1, v2.0 et v3 ; seule la v4 continue d'évoluer, et elle
exige Prism Central pc.2024.3 ou plus récent.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir des spécifications de l'API
    v4 de Nutanix (chemins et champs tels que les définissent les clients
    générés officiels) et testée contre des réponses enregistrées, pas contre
    un Prism Central en fonctionnement. Dites-nous ce qui casse.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_nutanix_`.

| Métrique | Labels | Signification |
|---|---|---|
| `clusters` | | Clusters, sans Prism Central lui-même. |
| `cluster_info` | `cluster`, `version` | Valeur 1 : la version d'AOS de chaque cluster. |
| `cluster_available` | `cluster` | 1 quand Prism Central joint le cluster. |
| `hosts` | | Hôtes de tous les clusters. |
| `host_info` | `host`, `cluster`, `hypervisor` | Valeur 1 : l'hyperviseur qu'exécute chaque hôte. |
| `host_cpu_cores`, `host_memory_bytes` | `host`, `cluster` | Capacité. |
| `host_degraded` | `host`, `cluster` | 1 quand Nutanix marque l'hôte comme dégradé. |
| `host_maintenance` | `host`, `cluster` | 1 quand l'hôte entre en mode maintenance ou y est. |
| `vms`, `vms_running` | | VM AHV, et combien sont allumées. |
| `vm_power_state_info` | `vm`, `cluster`, `state` | Valeur 1 : `on`, `off`, `paused` ou `undetermined`. |
| `vm_running` | `vm`, `cluster` | 1 quand la VM est allumée. |
| `vm_vcpus`, `vm_memory_bytes` | `vm`, `cluster` | Sockets × cœurs × threads, et mémoire configurée. |
| `storage_containers` | | Conteneurs de stockage, sans ceux, internes, de Nutanix. |
| `storage_container_capacity_bytes`, `storage_container_used_bytes`, `storage_container_used_percent` | `container`, `cluster` | Niveau de remplissage, d'après les dernières statistiques des trente dernières minutes. |
| `alerts_unresolved`, `alerts_critical`, `alerts_warning` | | Alertes non résolues, toutes et par gravité. |

Les [règles intégrées](../alerting/rules.md#nutanix) qui s'appliquent :

- **Hôte Nutanix dégradé** : un hôte est marqué dégradé depuis cinq minutes
  (Avertissement).
- **Conteneur de stockage Nutanix presque plein** : un conteneur est plein à
  90 % pendant quinze minutes (Avis).
- **Alertes critiques Nutanix** : Prism Central rapporte au moins une alerte
  critique non résolue depuis cinq minutes (Avertissement).

## Créer un utilisateur en lecture seule dans Prism Central

1. Dans Prism Central, ouvrez Admin Center > IAM > Identities et ajoutez un utilisateur local avec un long mot de passe aléatoire, nommé comme suit.

    ```
    dumbmonit
    ```

2. Sous Admin Center > IAM > Authorization Policies, créez une politique avec le rôle intégré Prism Viewer, pour toutes les entités, et attribuez-la à cet utilisateur. Prism Viewer lit les clusters, hôtes, VM, conteneurs de stockage et alertes, et ne modifie rien.

3. Dans DumbMonit, saisissez l'adresse de Prism Central, par exemple « prism-central.lan », avec ce nom d'utilisateur et ce mot de passe. Un seul Prism Central rapporte tous les clusters qui y sont enregistrés.

4. DumbMonit ne fait que lire l'API v4 (clustermgmt, vmm et monitoring, version 4.0) : il n'allume ni n'éteint jamais une VM, ne résout jamais une alerte et ne modifie jamais un conteneur de stockage.

!!! warning
    L'API v4 est servie par Prism Central pc.2024.3 ou plus récent ; un cluster géré par Prism Element seul, ou par un Prism Central plus ancien, n'est pas pris en charge. Prism Central fournit un certificat autosigné : cochez « Accepter un certificat non vérifiable » sauf si l'hôte DumbMonit fait confiance à son autorité de certification.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur et mot de passe | L'utilisateur local `dumbmonit` avec le rôle Prism Viewer. |

Adresse : le nom d'hôte ou l'IP de Prism Central, `host:port`, ou une URL.
Options :

- **Port** (9440).
- **Accepter un certificat non vérifiable** : le propre certificat de Prism
  Central est autosigné.
- **Délai par requête** (15 s).

## Dépannage

« Nutanix Prism Central refused the credentials … (401) » : mauvais nom
d'utilisateur ou mot de passe.

« Nutanix Prism Central has no /api/clustermgmt/v4.0/… » : l'adresse est celle
d'un Prism Element, ou d'un Prism Central antérieur à pc.2024.3.

Pas de `storage_container_used_percent` : le rôle n'autorise pas la lecture des
statistiques ; les conteneurs sont tout de même comptés, avec leur capacité
configurée.

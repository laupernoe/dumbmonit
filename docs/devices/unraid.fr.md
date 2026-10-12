# Unraid

Un serveur Unraid via son API GraphQL : un array qui n'est pas démarré, des
disques désactivés, manquants ou rapportant des erreurs de lecture, des
vérifications de parité qui ont trouvé des erreurs ou n'ont pas tourné
depuis des semaines, des pools de cache qui se remplissent, des conteneurs
qui devraient tourner et ne tournent pas, et l'état des VM.

DumbMonit envoie une clé d'API avec le rôle Viewer dans l'en-tête
`x-api-key` à `/graphql`. Le rôle Viewer lit tout et ne peut rien changer.
L'API est intégrée à Unraid depuis la 7.2 ; sur les versions 6.12 à 7.1 elle
vient avec le plugin Unraid Connect.

Chaque minute, DumbMonit envoie une requête obligatoire (version, array,
parité, disques de données et de cache) et quatre optionnelles, chacune à
part pour qu'un champ qu'une version plus ancienne de l'API n'a pas, ou des
VM éteintes, ne laisse de côté que cette partie : la vérification de parité,
les conteneurs Docker, les VM et les notifications non lues.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la documentation de l'API
    d'Unraid et du schéma GraphQL publié de l'API Unraid (noms de champs,
    énumérations, tailles en kilooctets), pas contre un serveur en
    fonctionnement. Chaque requête est une lecture. Dites-nous ce qui casse.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_unraid_`.

| Métrique | Labels | Signification |
|---|---|---|
| `version_info` | `version` | Valeur 1 : la version d'Unraid. |
| `array_started` | | 1 quand l'array est démarré. |
| `array_state_info` | `state` | Valeur 1 : `started`, `stopped`, `new_array`, `recon_disk`, `disable_disk`… |
| `array_size_bytes`, `array_used_bytes`, `array_free_bytes`, `array_used_percent` | | Capacité de l'array. |
| `disk_ok` | `disk`, `role` (`parity`, `data`, `cache`) | 1 quand le statut du disque est `DISK_OK`. Les emplacements vides sont laissés de côté. |
| `disk_status_info` | `disk`, `role`, `status` | Valeur 1 : le statut d'Unraid (`disk_ok`, `disk_dsbl`, `disk_np_missing`, `disk_invalid`…). |
| `disk_errors` | `disk`, `role` | Erreurs de lecture depuis le démarrage de l'array (la colonne Errors). |
| `disk_temperature_celsius` | `disk`, `role` | Seulement tant que le disque tourne : Unraid n'en rapporte aucune pour un disque endormi. |
| `disk_spinning` | `disk`, `role` | 1 quand le disque tourne. |
| `disk_size_bytes`, `disk_fs_size_bytes`, `disk_used_percent` | `disk`, `role` | Taille, taille du système de fichiers et remplissage, pour les disques avec un système de fichiers. |
| `disks`, `disks_problem`, `disks_with_errors` | | Compteurs. |
| `parity_check_status_info` | `status` | Valeur 1 : `completed`, `running`, `paused`, `cancelled`, `failed`, `never_run`. |
| `parity_check_running`, `parity_check_progress_percent` | | Une vérification en cours. |
| `parity_check_ok` | | Après une vérification terminée : 1 sans erreurs de synchro, 0 avec des erreurs ou en cas d'échec. Une vérification annulée ne prouve rien et ne fixe rien. |
| `parity_check_errors`, `parity_check_age_seconds`, `parity_check_duration_seconds` | | La dernière vérification. |
| `containers`, `containers_running`, `containers_autostart_stopped` | | Conteneurs Docker. |
| `container_running`, `container_autostart_stopped` | `container` | Par conteneur ; le second est 1 quand un conteneur réglé pour démarrer avec l'array ne tourne pas. |
| `vms`, `vms_running` | | VM. |
| `vm_state_info`, `vm_crashed` | `vm` (et `state`) | Par VM. |
| `notifications_unread` | `importance` : `alert`, `warning`, `info` | Notifications non lues. |

Les [règles intégrées](../alerting/rules.md#unraid) qui s'appliquent :

- **Array Unraid arrêté** : l'array n'est pas démarré depuis dix minutes
  (Avertissement).
- **Disque Unraid désactivé ou manquant** : le statut d'un disque n'est pas
  `DISK_OK` depuis cinq minutes (Avertissement). L'array tourne en mode
  dégradé.
- **Erreurs de lecture disque Unraid** : un disque rapporte des erreurs de
  lecture (Avis).
- **Disque Unraid trop chaud** : un disque de données ou de parité au-dessus
  de 55 °C pendant quinze minutes (Avis). Les disques de cache, souvent
  NVMe, sont laissés de côté.
- **Vérification de parité Unraid a trouvé des erreurs** (Avis).
- **Vérification de parité Unraid en retard** : la dernière vérification a
  plus de 40 jours (Avis).
- **Cache Unraid presque plein** : un disque de cache à plus de 90 % plein
  pendant trente minutes (Avis).
- **Conteneur Unraid arrêté** : un conteneur réglé pour démarrer avec
  l'array n'a pas tourné depuis dix minutes (Avis).

## La page de l'appareil

Le panneau Array dit ce qui ne va pas en une phrase, montre le remplissage de
l'array, la dernière vérification de parité, les conteneurs et VM en cours,
puis liste chaque disque — parité d'abord, puis données, puis cache — avec
son statut, sa température et son remplissage, et les conteneurs qui
devraient tourner.

## Créer une clé d'API en lecture seule pour DumbMonit

1. Sur Unraid 7.2 ou plus récent, l'API est intégrée. Sur Unraid 6.12 à 7.1, installez d'abord le plugin Unraid Connect depuis l'onglet Apps : il apporte la même API, et aucune connexion Unraid Connect n'est nécessaire pour un usage local.

2. Créez une clé d'API avec le rôle Viewer, qui lit tout et ne peut rien changer : sous Settings > Management Access > API Keys, ou depuis le terminal Unraid.

    ```
    unraid-api apikey --create --name dumbmonit --roles VIEWER --description "DumbMonit monitoring" --json
    ```

3. Copiez la clé. Dans DumbMonit, saisissez l'adresse du serveur Unraid, par exemple « tower.lan », et collez la clé comme Clé d'API. Si l'interface web est servie en HTTPS, choisissez https comme protocole.

!!! warning
    Unraid ne rapporte aucune température pour un disque arrêté, et DumbMonit n'en réveille jamais un : un disque endormi n'a simplement pas de température jusqu'à ce qu'il tourne à nouveau.

## Identifiants

| Identifiant | Champs |
|---|---|
| Clé d'API | Une clé avec le rôle Viewer, envoyée comme `x-api-key`. |

Adresse : le nom d'hôte ou l'IP du serveur Unraid, `host:port`, ou une URL.
Options :

- **Protocole** (`http`) et **Port** (80) : ceux de l'interface web. Avec
  SSL réglé sur Strict sous Settings > Management Access, utilisez `https`,
  et l'adresse `myunraid.net` si c'est celle du certificat.
- **Accepter un certificat non vérifiable** : pour un certificat autosigné.
- **Délai par requête** (15 s).

## Dépannage

« Unraid refused the API key (401) » : la clé est incorrecte ou a été
supprimée.

« Unraid accepted the API key but refused to answer (403) » : la clé n'a
aucun rôle, ou un rôle plus restreint que Viewer.

Aucune donnée de conteneur, VM ou parité : cette requête optionnelle a
échoué (VM éteintes, une version d'API plus ancienne sans le champ) ; le
reste est quand même lu. La raison est dans le journal du serveur au niveau
debug.

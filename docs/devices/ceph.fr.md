# Ceph

!!! warning "Bêta"

    Cette intégration a été construite à partir de la documentation de l'API
    REST du tableau de bord du gestionnaire Ceph, pas encore sur une grappe en
    fonctionnement. Elle ne fait que lire. Dites-nous ce qui ne marche pas.

Une grappe Ceph via l'API REST du module tableau de bord du gestionnaire :
l'état de santé et les contrôles qui le détaillent, les OSD arrêtés, sortis
ou presque pleins, les groupes de placement qui ne sont pas `active+clean`, la
capacité brute, les pools et les moniteurs sortis du quorum.

DumbMonit se connecte avec un utilisateur du tableau de bord au rôle intégré
`read-only` (`POST /api/auth`, les identifiants dans le corps de la requête),
puis envoie le jeton reçu en `Authorization: Bearer`, jamais dans l'URL.
Chaque appel demande `application/vnd.ceph.api.v1.0+json`, la version d'API que
documente le tableau de bord. Le jeton est gardé une heure et renouvelé quand
il est refusé : se déconnecter chaque minute remplirait la liste de
révocation des jetons du gestionnaire.

Une page est obligatoire — `/api/health/minimal` (santé, contrôles, groupes de
placement, capacité, débit client) — et quatre sont facultatives, laissées de
côté sans faire échouer le reste quand le rôle ou la version de Ceph ne les
sert pas : `/api/osd`, `/api/pool`, `/api/monitor` et `/api/summary` (la
version).

!!! note "Variante par l'agent"

    Sur un hôte qui a la CLI Ceph, `ceph status -f json` donne les mêmes
    chiffres de santé et de groupes de placement. DumbMonit ne le lit pas
    encore par l'agent ; l'API du tableau de bord ci-dessus est la voie prise
    en charge.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 8443 | TCP | API REST du tableau de bord (HTTPS, par défaut) |
| 8080 | TCP | Seulement avec l'option de protocole HTTP |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_ceph_`.

| Métrique | Labels | Signification |
|---|---|---|
| `version_info` | `version` | Valeur 1. |
| `health_status` | | 0 `HEALTH_OK`, 1 `HEALTH_WARN`, 2 `HEALTH_ERR`. |
| `health_ok` | | 1 quand l'état est `HEALTH_OK`. |
| `health_checks` | | Contrôles actifs, ceux qu'un opérateur a mis en sourdine exclus. |
| `health_check_info` | `check`, `severity` | Valeur 1 par contrôle actif (`OSD_DOWN`, `PG_DEGRADED`…). |
| `osds`, `osds_up`, `osds_in`, `osds_down` | | Nombres d'OSD. |
| `osd_up`, `osd_in` | `osd`, `host` | Par OSD. |
| `osd_used_percent` | `osd`, `host` | Remplissage de chaque OSD. |
| `pgs`, `pgs_clean`, `pgs_unclean` | | Groupes de placement ; propre veut dire `active` et `clean`. |
| `pgs_in_state` | `state` | Groupes de placement par état (`active+clean`, `active+undersized+degraded`…). |
| `objects`, `objects_degraded`, `objects_misplaced`, `objects_unfound` | | Nombres d'objets. |
| `capacity_bytes`, `capacity_used_bytes`, `capacity_available_bytes`, `capacity_used_percent` | | Capacité brute de la grappe. |
| `pools` | | Nombre de pools. |
| `pool_used_percent`, `pool_used_bytes`, `pool_objects` | `pool` | Par pool. |
| `mons`, `mons_in_quorum`, `mons_out_of_quorum` | | Moniteurs. |
| `hosts` | | Hôtes connus de l'orchestrateur. |
| `client_read_bytes_per_second`, `client_write_bytes_per_second`, `client_read_ops_per_second`, `client_write_ops_per_second`, `recovering_bytes_per_second` | | Débit des clients et de la récupération. |

Les [règles intégrées](../alerting/rules.md#ceph) qui s'appliquent :

- **Ceph health error** : `HEALTH_ERR` pendant cinq minutes (Avertissement).
- **Ceph health warning** : `HEALTH_WARN` pendant 30 minutes (Avis).
- **Ceph OSD down** : pendant dix minutes (Avis).
- **Ceph placement groups not clean** : pendant 30 minutes (Avis).
- **Ceph objects unfound** : pendant cinq minutes (Avertissement).
- **Ceph capacity high** : au-dessus de 80 % pendant 30 minutes (Avis).
- **Ceph OSD nearly full** : au-dessus de 85 % pendant 15 minutes (Avis).
- **Ceph pool nearly full** : au-dessus de 90 % pendant 15 minutes (Avis).
- **Ceph monitor out of quorum** : pendant cinq minutes (Avertissement).

## La page de la surveillance

Le panneau Cluster dit en une phrase ce qui ne va pas, montre les OSD
actifs, les groupes de placement propres, la capacité et les moniteurs dans le
quorum, puis liste les contrôles de santé actifs, les OSD qui demandent de
l'attention et les pools par remplissage.

## Créer un utilisateur du tableau de bord en lecture seule pour DumbMonit

1. Check that the dashboard is enabled and note the address of the active manager.

    ```
    ceph mgr services
    ```

2. Write a long random password to a file, and keep it: DumbMonit needs it.

    ```
    openssl rand -base64 24 | tr -d '\n' | tee /tmp/dumbmonit.pwd
    ```

3. Create the user with the built-in read-only role.

    ```
    ceph dashboard ac-user-create dumbmonit -i /tmp/dumbmonit.pwd read-only
    ```

4. Delete the password file.

    ```
    shred -u /tmp/dumbmonit.pwd
    ```

5. In DumbMonit, enter the address of the active manager (for example ceph.lan) with that user name and password.

(Les étapes ci-dessus reprennent le texte anglais affiché dans l'application.)

### Droits requis

- **read-only role** — Lit l'état de la grappe, ne change rien

!!! warning
    Un gestionnaire en attente redirige vers l'actif : utilisez l'adresse du gestionnaire actif. Le tableau de bord fournit un certificat auto-signé : cochez « Accepter un certificat invérifiable » sauf si vous avez installé le vôtre.

### Notes

- Le tableau de bord peut imposer une politique de mot de passe (longueur,
  complexité) ; un mot de passe aléatoire en base64 la satisfait normalement.
- Sans le module tableau de bord (`ceph mgr module enable dashboard`), il n'y
  a rien à lire.

## Identifiants

| Identifiant | Champs |
|---|---|
| Utilisateur et mot de passe du tableau de bord | L'utilisateur créé ci-dessus. |

Adresse : le nom d'hôte ou l'IP du gestionnaire actif, `hôte:port`, ou une
URL. Options :

- **Protocole** (https) : `http` ou `https`.
- **Port** (8443) : le port du tableau de bord, 8443 en HTTPS et 8080 en HTTP.
- **Accepter un certificat invérifiable** : le certificat d'origine du
  tableau de bord est auto-signé.
- **Délai par requête** (15 s).

## Dépannage

« Ceph refused the login » : nom d'utilisateur ou mot de passe incorrect, ou
utilisateur désactivé ou sans rôle.

« Ceph answered 303 … » : l'adresse est celle d'un gestionnaire en attente ;
utilisez l'actif, que montre `ceph mgr services`.

Aucun chiffre de pool, d'OSD ou de moniteur : ces pages sont facultatives et
le rôle ou la version de Ceph peut ne pas les servir ; la santé, les groupes de
placement et la capacité sont lus quand même.

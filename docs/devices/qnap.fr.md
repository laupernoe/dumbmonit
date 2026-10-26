# QNAP QTS / QuTS hero

!!! warning "Bêta"

    Cette intégration a été construite à partir des références publiques (les
    pages lues par l'intégration Home Assistant `qnap` et la bibliothèque
    `python-qnapstats`) et testée sur des réponses de forme enregistrée
    seulement, pas encore sur un vrai NAS QNAP. Elle ne fait que lire.
    Dites-nous ce qui ne marche pas.

Un NAS QNAP via les pages CGI de son interface web : des volumes qui se
remplissent, des disques dont l'état SMART n'est plus bon ou qui chauffent
trop, la température du processeur, la mémoire, et les mises à jour de QTS ou
QuTS hero que QNAP propose.

QNAP ne publie pas d'API REST documentée pour l'état du NAS : DumbMonit lit
donc les mêmes pages XML que l'interface web et que les clients de référence.
Chaque minute, il ouvre une session (`POST /cgi-bin/authLogin.cgi`, le mot de
passe encodé en base64 dans le corps de la requête, jamais dans l'URL), lit,
puis ferme la session (`/cgi-bin/authLogout.cgi`) même si une lecture a
échoué : QTS limite le nombre de sessions ouvertes. L'identifiant de session
voyage dans l'URL des lectures, DumbMonit le retire donc de tout message
d'erreur.

Une page est obligatoire — les informations système
(`manaRequest.cgi?subfunc=sysinfo` : processeur, mémoire, températures, durée
de fonctionnement) — et trois sont facultatives, laissées de côté sans faire
échouer le reste quand le compte ou la version de QTS ne les sert pas : les
volumes (`chartReq.cgi`), les disques et leur état SMART (`disk_manage.cgi`) et
la recherche de mise à jour (`sysRequest.cgi`).

L'état des groupes RAID (dégradé, reconstruction) n'est pas lu : aucun client
de référence ne le documente.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | Interface web (HTTPS, par défaut) |
| 8080 | TCP | Seulement avec l'option de protocole HTTP |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_qnap_`.

| Métrique | Labels | Signification |
|---|---|---|
| `version_info` | `version`, `build`, `model`, `hostname` | Valeur 1. |
| `cpu_usage_percent` | | Usage du processeur. |
| `cpu_temperature_celsius`, `system_temperature_celsius` | | Température du processeur et du boîtier. |
| `memory_total_bytes`, `memory_used_percent` | | Mémoire. |
| `uptime_seconds` | | Depuis le dernier démarrage. |
| `volumes` | | Nombre de volumes. |
| `volume_size_bytes`, `volume_free_bytes`, `volume_used_percent` | `volume` | Par volume. |
| `disks`, `disks_unhealthy` | | Disques présents (les baies vides sont ignorées) et ceux dont l'état SMART n'est pas bon. |
| `disk_ok` | `disk`, `model` | 1 quand l'état SMART est bon. |
| `disk_warning` | `disk`, `model` | 1 quand c'est un avertissement plutôt qu'une défaillance. |
| `disk_temperature_celsius` | `disk`, `model` | Température du disque. |
| `firmware_update_available` | | 1 quand QNAP propose une version plus récente. |
| `firmware_latest_info` | `version` | Valeur 1 : cette version. |

Les [règles intégrées](../alerting/rules.md#qnap-qts) qui s'appliquent :

- **QNAP volume nearly full** : au-dessus de 90 % pendant 30 minutes (Avis).
- **QNAP disk health not good** : pendant 15 minutes (Avis).
- **QNAP disk too hot** : au-dessus de 55 °C pendant 15 minutes (Avis).
- **QNAP processor too hot** : au-dessus de 90 °C pendant 15 minutes (Avis).
- **QNAP firmware update** : une version est proposée (Info, rappel
  hebdomadaire).

## La page de la surveillance

Le panneau du NAS dit en une phrase ce qui ne va pas, montre le processeur,
la mémoire, la température et la durée de fonctionnement, puis liste les
volumes (les plus pleins d'abord) et les disques avec leur état SMART et leur
température.

## Créer un compte limité pour DumbMonit

1. Open Control Panel → Privilege → Users → Create → Create a User and create the account as follows, with a long random password. Tick no share and no application privilege.

    ```
    dumbmonit
    ```

2. Add the account to the administrators group: QTS serves the system status, volumes and disks to that group only.

3. Optional: under Control Panel → System → Security → Allow/Deny List, allow the web interface only from the DumbMonit server's address.

4. In DumbMonit, enter the NAS address (for example nas.lan) with that user name and password. Choose http and port 8080 only if HTTPS is off.

(Les étapes ci-dessus reprennent le texte anglais affiché dans l'application,
dont les menus de QTS portent le nom anglais.)

### Droits requis

- **administrators group** — Le seul groupe que QTS autorise à lire l'état du système
- **No share, no application privilege** — Rien d'autre n'est nécessaire

!!! warning
    QTS n'a pas de rôle en lecture seule pour l'état du système : le compte est donc dans le groupe administrators. Donnez-lui un mot de passe unique, et un compte sans vérification en deux étapes. En HTTP, le mot de passe voyage encodé en base64, pas chiffré : préférez HTTPS.

### Notes

- DumbMonit n'écrit jamais : chaque appel est une lecture, et le compte n'a
  aucun droit de partage ni d'application.
- Un compte avec vérification en deux étapes ne peut pas se connecter pour une
  supervision : créez-en un dédié, sans elle.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur et mot de passe | Le compte créé ci-dessus. |

Adresse : le nom d'hôte ou l'IP du NAS, `hôte:port`, ou une URL. Options :

- **Protocole** (https) : `http` ou `https`.
- **Port** (443) : utilisé si l'adresse ne donne pas de port ; saisissez 8080
  pour HTTP, QTS n'utilise pas le 80.
- **Accepter un certificat invérifiable** : le certificat d'origine du NAS est
  auto-signé.
- **Délai par requête** (15 s).

## Dépannage

« QNAP refused the login » : nom d'utilisateur ou mot de passe incorrect, ou
compte désactivé.

« The account uses 2-step verification » : créez un compte dédié sans elle.

« QNAP refused the session … the account may lack the right to read it » : le
compte n'est pas dans le groupe administrators.

Aucun disque ni volume : ces pages sont facultatives et une version de QTS peut
les servir autrement ; le reste est lu quand même. Dites-nous votre version de
QTS.

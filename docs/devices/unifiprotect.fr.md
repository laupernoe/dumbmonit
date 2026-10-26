# UniFi Protect (beta)

Si les caméras et leur enregistreur font leur travail : caméras hors ligne, caméras connectées qui n'enregistrent pas, disques du NVR en défaut, stockage utilisé et enregistrement coupé. À ne pas confondre avec le moniteur UniFi Network (bornes, commutateurs, passerelle).

!!! note "Bêta"
    Écrit d'après la documentation d'API de l'éditeur et testé sur des réponses réalistes enregistrées, pas encore sur une instance réelle. Dites-nous ce que vous observez.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | UniFi OS (HTTPS, par défaut) |

La panne à surveiller : **une caméra qui a décroché sans que personne ne le voie**. Un câble, un port PoE ou une mise à jour de firmware, et une caméra disparaît du NVR ; les images dont vous avez besoin sont celles qui n'ont jamais été enregistrées. Une caméra connectée qui n'enregistre pas est la même perte, un cran plus loin.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_unifiprotect_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `cameras_total`, `cameras_offline`, `cameras_not_recording` | caméras adoptées ; celles non connectées ; celles connectées, réglées pour enregistrer, mais qui n'enregistrent pas |  |
| `camera_online`, `camera_recording` | 1 par caméra tant qu'elle est connectée, tant qu'elle enregistre (32 au plus) | `camera` |
| `storage_used_percent`, `storage_size_bytes` | stockage du NVR utilisé et sa capacité |  |
| `disks_total`, `disks_unhealthy` | disques du NVR, et ceux signalés en mauvais état |  |
| `recording_disabled` | 1 quand l'enregistrement est coupé sur le NVR |  |

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui s'appliquent : Protect camera offline, Protect camera not recording, Protect NVR disk unhealthy, Protect recording disabled, plus Appareil inaccessible.

## Créer un utilisateur local en lecture seule pour Protect

1. Dans UniFi OS, ouvrez Admins & Users → Create New et créez un utilisateur local nommé comme suit, sans compte cloud.

    ```
    dumbmonit
    ```

2. Donnez-lui l'accès à l'application Protect seulement, avec le rôle View Only, et aucun à Network ni aux autres applications.

3. Dans DumbMonit, saisissez l'adresse de la console et ce nom d'utilisateur et mot de passe.

!!! warning
    Utilisez un compte local : UniFi OS refuse ici les comptes cloud Ubiquiti. Le certificat de la console est le plus souvent auto-signé : activez l'option de certificat.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur / mot de passe | Un utilisateur local UniFi OS. DumbMonit se connecte sur `/api/auth/login`, garde le cookie de session entre deux sondes et se reconnecte à son expiration. |

Adresse : le nom d'hôte de la console (`unvr.lan`) ou `host:port`. Le stockage utilisé reste élevé par construction : Protect écrase ses plus anciennes images, aucune règle ne se déclenche donc sur un disque plein. Options : protocole (HTTPS par défaut), port (443), vérification du certificat et délai de requête (15 s).

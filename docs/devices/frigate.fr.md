# Frigate NVR (beta)

Si Frigate voit encore : caméras qui ont cessé de livrer des images, vitesse d'inférence et blocages des détecteurs (Coral, GPU, CPU), débits d'images, disque des enregistrements et mises à jour.

!!! note "Bêta"
    Écrit d'après la documentation d'API de l'éditeur et testé sur des réponses réalistes enregistrées, pas encore sur une instance réelle. Dites-nous ce que vous observez.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 5000 | TCP | API (HTTP, par défaut, sans connexion) |
| 8971 | TCP | Seulement avec le port authentifié derrière un proxy à jeton porteur |

La panne à surveiller : **une caméra qui n'envoie plus d'images**. Frigate ne le montre que dans son interface ; ses enregistrements et détections s'arrêtent simplement. `camera_fps` à zéro est le signe le plus fiable. Un détecteur figé sur une inférence, ou un disque d'enregistrement plein, sont les deux autres.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_frigate_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `update_available` | 1 quand Frigate connaît une version plus récente | `latest_version` |
| `uptime_seconds`, `detection_fps` | temps depuis le démarrage de Frigate ; détections par seconde, toutes caméras |  |
| `cameras_total`, `cameras_offline` | caméras, et celles qui ne livrent aucune image |  |
| `camera_online`, `camera_fps` | 1 tant que des images arrivent ; images par seconde reçues (32 caméras au plus) | `camera` |
| `camera_process_fps`, `camera_skipped_fps`, `camera_detection_fps` | images traitées, jetées faute de temps, et envoyées à la détection | `camera` |
| `detector_inference_ms` | durée d'inférence de chaque détecteur | `detector` |
| `detector_busy_seconds` | depuis combien de temps un détecteur est sur la même inférence (0 au repos) | `detector` |
| `storage_used_percent`, `storage_free_bytes` | disques d'enregistrement (les montages en mémoire sont ignorés) | `path` |

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui s'appliquent : Frigate camera offline, Frigate detector stuck, Frigate storage almost full, Frigate update available, plus Appareil inaccessible.

## Laisser DumbMonit lire les statistiques de Frigate

1. Rien à installer : vérifiez depuis l'hôte DumbMonit que l'API de Frigate répond sur le port 5000.

    ```
    curl http://frigate.lan:5000/api/version
    ```

2. Si un proxy devant Frigate attend un jeton (le port 8971 exige une connexion), choisissez Jeton porteur ci-dessous ; sinon, choisissez Aucune authentification.

3. Dans DumbMonit, saisissez l'adresse de Frigate, par exemple « frigate.lan ».

!!! warning
    Le port 5000 n'a aucune connexion et donne le plein contrôle de Frigate : ne l'exposez qu'à DumbMonit, jamais à internet. Une caméra que vous avez désactivée dans les réglages de Frigate apparaît aussi hors ligne.

## Identifiants

| Identifiant | Champs |
|---|---|
| Aucune authentification | Le port 5000, l'API interne. |
| Jeton porteur | Envoyé en `Authorization: Bearer …`, pour un proxy qui l'attend. |

Adresse : un nom d'hôte (`frigate.lan`), `host:port`, ou une URL complète avec préfixe de chemin. Options : protocole (HTTP par défaut), port (5000), vérification du certificat et délai de requête (15 s).

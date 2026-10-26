# Radarr (beta)

Si Radarr trouve et importe encore les films : erreurs de santé, clients de téléchargement et indexeurs en échec, erreurs de la file de téléchargement, espace disque et mises à jour.

!!! note "Bêta"
    Écrit d'après la documentation d'API de l'éditeur et testé sur des réponses réalistes enregistrées, pas encore sur une instance réelle. Dites-nous ce que vous observez.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 7878 | TCP | API (HTTP, par défaut) |
| 443 | TCP | Seulement avec le schéma https derrière un reverse proxy |

La panne à surveiller : **un client de téléchargement ou un indexeur qui s'est arrêté sans bruit**. Radarr continue de tourner et sa page s'ouvre, mais plus rien n'est récupéré ni importé ; la seule trace est le bandeau rouge de sa page System → Status, que DumbMonit lit tel quel.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_radarr_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `update_available` | 1 quand une version plus récente existe (absent quand la liste des mises à jour est indisponible) | `latest_version` |
| `health_errors`, `health_warnings` | contrôles de la page System → Status qui signalent une erreur ou un avertissement |  |
| `health_issue` | valeur 1 par erreur ou avertissement, 32 au plus | `source`, `level` |
| `queue_total`, `queue_unknown` | éléments de la file de téléchargement, dont ceux que Radarr ne sait pas rattacher à sa bibliothèque |  |
| `queue_has_errors`, `queue_has_warnings` | 1 quand au moins un élément de la file a une erreur (téléchargement échoué, import bloqué) ou un avertissement |  |
| `disk_used_percent`, `disk_free_bytes` | chaque disque que Radarr signale (dossiers racine, dossier de téléchargement), 32 au plus | `path` |
| `scrape_errors` | appels qui ont échoué à la dernière sonde |  |

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui s'appliquent : Radarr health error, Radarr queue has errors, Radarr disk almost full, Radarr update available, plus Appareil inaccessible.

## Lire la clé d'API dans Radarr

1. Dans Radarr, ouvrez Settings → General → Security et copiez l'API Key.

2. Dans DumbMonit, saisissez l'adresse de Radarr et collez la clé.

!!! warning
    Une clé d'API Radarr ne peut pas être limitée à la lecture : ne la gardez que dans DumbMonit, qui n'envoie que des requêtes GET.

## Identifiants

| Identifiant | Champs |
|---|---|
| Clé d'API | Envoyée dans l'en-tête `X-Api-Key`. |

Adresse : un nom d'hôte (`radarr.lan`), `host:port`, ou une URL complète avec préfixe de chemin (`https://home.lan/radarr`, selon le réglage URL Base de Radarr). Options : protocole (HTTP par défaut), port (7878), vérification du certificat et délai de requête (15 s).

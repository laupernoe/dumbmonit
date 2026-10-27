# Prowlarr (beta)

Si Prowlarr sert encore ses indexeurs : erreurs de santé, indexeurs désactivés après des échecs, espace disque et mises à jour.

!!! note "Bêta"
    Écrit d'après la documentation d'API de l'éditeur et testé sur des réponses réalistes enregistrées, pas encore sur une instance réelle. Dites-nous ce que vous observez.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 9696 | TCP | API (HTTP, par défaut) |
| 443 | TCP | Seulement avec le schéma https derrière un reverse proxy |

La panne à surveiller : **un indexeur désactivé puis oublié**. Après quelques échecs, Prowlarr désactive un indexeur pour des heures, puis des jours ; Sonarr, Radarr et Lidarr cherchent avec les autres et rien ne dit que des versions manquent.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_prowlarr_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `update_available` | 1 quand une version plus récente existe (absent quand la liste des mises à jour est indisponible) | `latest_version` |
| `health_errors`, `health_warnings` | contrôles de la page System → Status qui signalent une erreur ou un avertissement |  |
| `health_issue` | valeur 1 par erreur ou avertissement, 32 au plus | `source`, `level` |
| `indexers_disabled` | indexeurs encore désactivés après des échecs répétés |  |
| `disk_used_percent`, `disk_free_bytes` | chaque disque que Prowlarr signale (dossiers racine, dossier de téléchargement), 32 au plus | `path` |
| `scrape_errors` | appels qui ont échoué à la dernière sonde |  |

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui s'appliquent : Prowlarr health error, Prowlarr indexers disabled, Prowlarr disk almost full, Prowlarr update available, plus Appareil inaccessible.

## Lire la clé d'API dans Prowlarr

1. Dans Prowlarr, ouvrez Settings → General → Security et copiez l'API Key.

2. Dans DumbMonit, saisissez l'adresse de Prowlarr et collez la clé.

!!! warning
    Une clé d'API Prowlarr ne peut pas être limitée à la lecture : ne la gardez que dans DumbMonit, qui n'envoie que des requêtes GET.

## Identifiants

| Identifiant | Champs |
|---|---|
| Clé d'API | Envoyée dans l'en-tête `X-Api-Key`. |

Adresse : un nom d'hôte (`prowlarr.lan`), `host:port`, ou une URL complète avec préfixe de chemin (`https://home.lan/prowlarr`, selon le réglage URL Base de Prowlarr). Options : protocole (HTTP par défaut), port (9696), vérification du certificat et délai de requête (15 s).

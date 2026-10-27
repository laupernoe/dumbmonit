# Vaultwarden (beta)

Si le coffre de mots de passe répond, quelle version tourne, si n'importe qui peut encore créer un compte et, avec le jeton d'administration, combien de comptes existent, sont désactivés ou n'ont pas de connexion en deux étapes.

!!! note "Bêta"
    Écrit d'après la documentation d'API de l'éditeur et testé sur des réponses réalistes enregistrées, pas encore sur une instance réelle. Dites-nous ce que vous observez.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | API (HTTPS, par défaut) |
| 80 | TCP | Seulement avec l'option de schéma http, dans un réseau de confiance |

Un coffre qui ne répond pas est la panne qui compte, et c'est le simple contrôle `/alive` qui la voit : les applications gardent une copie hors ligne un moment, personne ne s'en aperçoit donc avant qu'un nouvel appareil ait besoin de se synchroniser. Les inscriptions ouvertes sont le problème plus discret qui mérite un coup d'œil sur un coffre exposé à internet.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_vaultwarden_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `up` | valeur 1 tant que `/alive` répond |  |
| `version_info` | valeur 1 | `version` |
| `signups_open` | 1 quand n'importe qui peut créer un compte |  |
| `admin_readable` | 1 quand la page d'administration a été lue (absent sans le jeton d'administration) |  |
| `users`, `users_disabled`, `users_without_2fa` | comptes ; comptes désactivés ; comptes actifs sans connexion en deux étapes |  |
| `items` | éléments de coffre, tous comptes confondus |  |
| `scrape_errors` | appels qui ont échoué à la dernière sonde |  |

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui s'appliquent : Vaultwarden sign-ups open, plus Appareil inaccessible.

## Surveiller Vaultwarden, et lire en option sa page d'administration

1. Rien à installer : vérifiez depuis l'hôte DumbMonit que Vaultwarden répond.

    ```
    curl https://vault.example.com/alive
    ```

2. Facultatif, pour les compteurs de comptes : activez la page d'administration avec un ADMIN_TOKEN sur le serveur, et gardez le jeton en clair que vous saisissez sur la page /admin.

3. Dans DumbMonit, saisissez l'adresse de Vaultwarden ; collez le jeton d'administration, ou choisissez Sans jeton d'administration pour les contrôles publics seulement.

!!! warning
    Le jeton d'administration ouvre la page d'administration de Vaultwarden et peut changer tous les réglages : ne le stockez que dans DumbMonit, ou omettez-le si vous n'avez pas besoin des compteurs. DumbMonit ne lit que des compteurs, jamais un nom, une adresse e-mail ou un élément de coffre.

## Identifiants

| Identifiant | Champs |
|---|---|
| Sans jeton d'administration | Contrôles publics seulement : `/alive` et `/api/config`. |
| Jeton d'administration | L'ADMIN_TOKEN en clair. Vaultwarden n'a pas de clé d'API d'administration : le jeton ouvre une session sur `/admin` (cookie valable 20 minutes), que DumbMonit garde 15 minutes entre deux sondes, car la connexion est limitée en débit. |

Adresse : un nom d'hôte (`vault.example.com`), `host:port`, ou une URL complète avec préfixe de chemin. Options : protocole (HTTPS par défaut), port (443), vérification du certificat et délai de requête (15 s).

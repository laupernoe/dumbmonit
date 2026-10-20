# Apache httpd

Le serveur web : workers occupés et au repos, le scoreboard des workers par
état, et les débits de requêtes/octets quand ExtendedStatus est activé.

DumbMonit lit le `mod_status` intégré d'Apache, interrogé avec `?auto` pour
qu'il réponde en simples lignes `Clé: Valeur` plutôt que par la page HTML
destinée à un navigateur.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_apache_`. Les métriques `_total`
sont des compteurs : graphiques et règles les transforment en débits. Tout ce
qui est sous `workers_busy`/`workers_idle` et `scoreboard` demande
`ExtendedStatus On`.

| Métrique | Quoi | Labels |
|---|---|---|
| `workers_busy`, `workers_idle` | processus ou threads worker, par état | |
| `scoreboard` | emplacements worker par état de scoreboard | `state` |
| `requests_total` | requêtes servies depuis le démarrage d'Apache | |
| `sent_bytes_total` | octets envoyés depuis le démarrage d'Apache | |
| `uptime_seconds` | temps depuis le démarrage d'Apache | |
| `requests_per_second`, `bytes_per_second`, `bytes_per_request` | propres moyennes courantes d'Apache | |
| `cpu_load_ratio` | propre valeur `CPULoad` d'Apache | |
| `connections_total`, `connections_async_writing`, `connections_async_keepalive`, `connections_async_closing` | compteurs de connexions asynchrones (MPM event et worker) | |
| `version_info` | présence ; l'en-tête de réponse `Server` | `version` |

`scoreboard` s'additionne toujours au nombre configuré d'emplacements worker :
les états sont waiting, open_slot, starting, reading, sending, keepalive,
dns_lookup, closing, logging, finishing et idle_cleanup.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais Apache. Il montre les workers occupés et au repos, la
répartition du scoreboard, et, avec ExtendedStatus, les débits de requêtes et
d'octets sur 24 heures.

## Activer le mod_status d'Apache

1. Activez mod_status (il est fourni avec Apache et souvent déjà actif).

    ```
    a2enmod status
    ```

2. Ajoutez ce location à la configuration d'Apache, restreint à l'hôte DumbMonit (adaptez l'adresse).

    ```
    <Location "/server-status">
    SetHandler server-status
    Require ip 10.0.0.5
    </Location>
    ```

3. Facultatif : activez les compteurs de requêtes et d'octets.

    ```
    ExtendedStatus On
    ```

4. Rechargez Apache, puis ouvrez http://apache.lan/server-status?auto depuis l'hôte DumbMonit.

    ```
    apachectl graceful
    ```

5. Dans DumbMonit, saisissez l'hôte Apache, par exemple apache.lan, et le chemin s'il n'est pas /server-status.

!!! warning
    Sans ExtendedStatus, requêtes et octets ne sont pas rapportés du tout : le graphique reste vide plutôt que plat.

### Pour aller plus loin

Sur Debian et Ubuntu, le location va dans un fichier à part ; ailleurs, dans la configuration principale. DumbMonit ajoute ?auto lui-même. Sans ExtendedStatus, seuls les workers occupés et inactifs et le scoreboard sont rapportés. Apache ne publie pas sa propre version dans mod_status ; DumbMonit la lit dans l'en-tête de réponse Server, que certaines distributions réduisent volontairement.

Documentation de l'éditeur : https://httpd.apache.org/docs/2.4/mod/mod_status.html

## Identifiants

| Identifiant | Champs |
|---|---|
| Pas de mot de passe | Rien : la page de statut est filtrée pour que seul DumbMonit l'atteigne. |
| Authentification basique | L'utilisateur et le mot de passe du bloc protégeant la page de statut, s'il y en a. |

Adresse : un nom d'hôte ou une IP, avec le port s'il n'est pas 80. Options :
protocole (HTTP par défaut), port, chemin de statut (`/server-status` par
défaut, `?auto` ajouté automatiquement), vérification du certificat et délai
de requête (10 s).

# Pi-hole

Le bloqueur de publicité DNS : s'il bloque, combien de requêtes il répond et
bloque, l'ancienneté de ses listes de blocage, si ses résolveurs amont
échouent, si une mise à jour attend, et les messages de sa propre page de
diagnostic.

DumbMonit lit l'API REST de Pi-hole v6 (`/api`). Pi-hole 5 et antérieur n'ont
pas cette API et ne sont pas supportés.

Ce qui tourne mal avec un Pi-hole se montre rarement : le DNS continue de
répondre. Le blocage est mis en pause « pendant cinq minutes » et ne revient
jamais, la mise à jour hebdomadaire des listes de blocage (gravity) a échoué
depuis un mois, un résolveur amont répond SERVFAIL, et la page de diagnostic
que personne n'ouvre en donnait la raison depuis le début.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_pihole_`. Pi-hole compte les
requêtes sur les dernières 24 heures, une fenêtre glissante qui monte et
descend : ces chiffres sont des jauges, suffixées `_24h`, pas des compteurs.

| Métrique | Quoi | Labels |
|---|---|---|
| `blocking_enabled` | 1 quand le blocage est actif ; 0 quand il est désactivé, en pause ou en échec | |
| `blocking_timer_seconds` | temps restant avant qu'un blocage en pause se réactive seul | |
| `queries_24h`, `queries_blocked_24h`, `queries_blocked_percent` | requêtes répondues et bloquées dans les dernières 24 heures | |
| `queries_forwarded_24h`, `queries_cached_24h`, `unique_domains_24h` | requêtes envoyées en amont, répondues depuis le cache, domaines distincts | |
| `queries_per_second` | moyenne sur la dernière minute | |
| `replies_servfail_24h` | réponses SERVFAIL dans les dernières 24 heures : un résolveur amont en échec se montre ici d'abord | |
| `clients_active`, `clients_total` | clients vus | |
| `gravity_domains` | domaines sur les listes de blocage | |
| `gravity_last_update_timestamp_seconds`, `gravity_age_seconds` | dernière reconstruction des listes de blocage, et depuis quand | |
| `version_info` | valeur 1 | `core`, `web`, `ftl`, `docker` |
| `update_available` | 1 quand une version plus récente d'un composant est publiée, 0 sinon ; absent jusqu'à ce que la vérification quotidienne de Pi-hole ait tourné | `component` (`core`, `web`, `ftl`, `docker`) |
| `updates_available` | mises à jour à installer : l'image en Docker (core, web et FTL viennent avec), sinon chaque composant obsolète | |
| `messages`, `messages_by_type` | messages sur la page de diagnostic | `type` (`LOAD`, `RATE_LIMIT`, `LIST`…) |
| `ftl_uptime_seconds`, `ftl_memory_percent`, `ftl_cpu_percent` | le démon FTL | |
| `privacy_level` | le niveau de confidentialité de Pi-hole, 0 à 3 | |
| `scrape_errors` | appels d'API secondaires qui ont échoué pendant la dernière vérification | |

Sur une branche de développement, Pi-hole compare des commits plutôt que des
numéros de version, et DumbMonit fait de même.

Les [règles intégrées](../alerting/rules.md#pi-hole) qui s'appliquent :
blocage Pi-hole désactivé, listes de blocage Pi-hole obsolètes, mise à jour
Pi-hole disponible, messages de diagnostic Pi-hole, plus Appareil
inaccessible.

## La page de l'appareil

Le panneau au-dessus des graphiques lit ce que la vérification a stocké ;
ouvrir la page ne se connecte jamais à Pi-hole. Il montre si le blocage est
actif (et quand une pause se termine), les requêtes et la part bloquée dans
les dernières 24 heures, la taille et l'âge des listes de blocage, les
clients, les versions avec toute mise à jour en attente, et le nombre de
messages de diagnostic.

## Créer un mot de passe d'application dans Pi-hole

1. Pi-hole v6 ou plus récent est requis : la version 5 n'avait pas d'API REST. Pi-hole n'a pas de compte en lecture seule ; le plus proche est un mot de passe d'application, qui ne peut changer aucun réglage.

2. Dans l'interface web de Pi-hole, ouvrez Settings → Web interface / API et passez la page de Basic à Expert. Cliquez sur Configure app password, copiez le mot de passe affiché, puis cliquez sur Enable new app password. Pi-hole ne le montre qu'une fois, et un nouveau remplace le précédent.

3. Un mot de passe d'application reste en lecture seule pour les réglages tant que webserver.api.app_sudo est false, le défaut. Vérifiez-le sur la machine Pi-hole (en Docker, préfixez la commande avec docker exec et le nom du conteneur) :

    ```
    pihole-FTL --config webserver.api.app_sudo
    ```

4. Dans DumbMonit, saisissez l'adresse de Pi-hole, par exemple « pi.hole » ou « 192.168.1.53 », et collez le mot de passe d'application. Pour un Pi-hole dont l'interface web n'a pas de mot de passe, choisissez Pas de mot de passe à la place.

5. DumbMonit ne fait que lire. Il ne met jamais le blocage en pause et ne change jamais une liste, et il garde une session d'API ouverte entre les vérifications plutôt que d'en ouvrir une par vérification : Pi-hole autorise 16 sessions à la fois.

!!! warning
    Le mot de passe d'application permet quand même de mettre le blocage en
    pause et de modifier des listes via l'API : réservez-le à DumbMonit
    seulement. Pi-hole sert du HTTP simple sur le port 80, et le mot de passe
    traverse le réseau chaque fois qu'une session s'ouvre : sur un réseau non
    sûr, utilisez HTTPS sur le port 443.

Un mot de passe d'application se connecte aussi sans le second facteur quand
la double authentification est activée : c'est à ça qu'il sert. Pour le
révoquer, videz `webserver.api.app_pwhash` sous Settings → All settings, ou
activez-en un nouveau : le précédent arrête de fonctionner immédiatement.

## Identifiants

| Identifiant | Champs |
|---|---|
| Mot de passe d'application | Le mot de passe généré ci-dessus. DumbMonit l'échange contre une session, envoyée dans l'en-tête `X-FTL-SID`, et réutilise cette session tant que Pi-hole la garde (30 minutes après sa dernière utilisation). |
| Pas de mot de passe | Seulement pour un Pi-hole dont l'interface web n'a pas de mot de passe. |

Adresse : un nom d'hôte ou une IP (`pi.hole`), `host:port`, ou une URL
complète (`https://pi.hole`) ; le chemin de l'interface web ou de l'API,
copié depuis le navigateur, est ignoré. Options : protocole (HTTP par
défaut), port (80), vérification du certificat (pour HTTPS sur 443, le propre
certificat de Pi-hole est autosigné) et délai de requête (10 s).

Si les vérifications échouent avec « no free API session », les sessions de
Pi-hole sont toutes prises, généralement par un script qui se connecte à
chaque exécution sans se déconnecter. Elles expirent après 30 minutes ;
`webserver.api.max_sessions` relève la limite.

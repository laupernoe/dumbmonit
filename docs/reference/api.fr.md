# API HTTP

Tout ce que fait l'UI web passe par `/api/*`, et la même API est ouverte à
vos propres scripts, tableaux de bord et assistants : il n'y a pas de moitié
privée. C'est du JSON simple sur HTTP, authentifié par un cookie de session
(l'UI web) ou par un jeton d'API (tout le reste).

!!! warning "Avant la 1.0"
    DumbMonit est avant la 1.0 et son API aussi : routes et champs peuvent
    encore changer. La version d'API (ci-dessous) change chaque fois qu'une
    route ou un champ est supprimé ou renommé, pour qu'un client puisse le
    savoir.

## Description OpenAPI

Chaque instance décrit sa propre API dans un document
[OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.0) :

```
GET /api/openapi.json
```

Il est public — il ne porte aucun secret, aucune adresse et rien sur vos
appareils — et c'est la référence exhaustive : chaque route qu'un client
peut appeler, ses paramètres, les formes de requête et de réponse, les
réponses d'erreur et la portée dont elle a besoin (l'extension
`x-token-scope` sur chaque opération : `public`, `read`, `write`, ou `none`
pour les routes qu'un jeton ne peut jamais appeler). Les tableaux plus bas
sur cette page sont un résumé lisible de la même chose.

Laissés de côté volontairement : le protocole d'agent (`/api/ingest`,
`/api/agent/commands*`, `/api/agent/relay*`), que seul l'[agent](../devices/agent.md)
parle ; les redirections de navigateur de la connexion SSO
(`/api/auth/oidc/start`, `/api/auth/oidc/callback`) ; et les
téléchargements d'agent (`/install.sh`, `/install.ps1`, `/download/*`).

Chargez-le dans n'importe quel outil OpenAPI :

```bash
# Le parcourir, et essayer des appels avec votre jeton, dans Swagger UI
docker run --rm -p 8081:8080 \
  -e SWAGGER_JSON_URL=http://monit.lan:8080/api/openapi.json swaggerapi/swagger-ui
# puis ouvrez http://localhost:8081 et collez "dmt_…" sous Authorize → bearerAuth

# Générer une bibliothèque client (ici Python)
docker run --rm -v "$PWD:/local" openapitools/openapi-generator-cli generate \
  -i http://monit.lan:8080/api/openapi.json -g python -o /local/dumbmonit-client

# Ou juste lister ce qu'il y a
curl -s http://monit.lan:8080/api/openapi.json | jq -r '.paths | keys[]'
```

Postman et Insomnia l'importent depuis la même URL.

## Authentification {#authentication}

Deux voies d'entrée, acceptées sur chaque route protégée :

- **Cookie de session** — ce que l'UI web utilise. Connectez-vous avec un
  mot de passe (ou SSO) et envoyez le cookie HttpOnly `dumbmonit_session`
  (SameSite Lax ; `Secure` quand `DUMBMONIT_COOKIE_SECURE=1` ; 30 jours).
  Chaque requête qui change l'état (`POST`, `PUT`, `DELETE`) doit aussi
  porter `X-Requested-With: DumbMonit` — la preuve anti-CSRF que l'UI
  ajoute automatiquement ; sans elle la requête est refusée avec `403`.
- **Jeton d'API** — `Authorization: Bearer dmt_…`, pour les scripts,
  tableaux de bord et assistants. Créé dans Réglages → **API & assistants**
  (les mêmes jetons que le [serveur MCP](../using/assistant.md) utilise) et
  montré une fois. Aucun cookie n'est impliqué, aucun en-tête
  `X-Requested-With` n'est donc jamais nécessaire.

```bash
# Jeton d'API : un en-tête, rien d'autre
curl -H 'Authorization: Bearer dmt_…' http://monit.lan:8080/api/targets
curl -H 'Authorization: Bearer dmt_…' -X POST http://monit.lan:8080/api/targets/4/probe

# Session : se connecter (204 et un en-tête Set-Cookie), puis envoyer le cookie
curl -c cookies.txt -X POST http://monit.lan:8080/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"admin","password":"…"}'
curl -b cookies.txt http://monit.lan:8080/api/targets
curl -b cookies.txt -X POST http://monit.lan:8080/api/targets \
  -H 'X-Requested-With: DumbMonit' -H 'content-type: application/json' \
  -d '{"name":"NAS","address":"nas.lan","kind":"snmp","credential":{"type":"snmp_community","community":"public"}}'
```

Quand une requête porte un jeton bearer, le cookie est ignoré : un jeton
révoqué est refusé même depuis un navigateur connecté, plutôt que de
retomber silencieusement sur la session.

Les jetons d'enrôlement d'agent (`dmon_…`) sont autre chose : ils ne
fonctionnent que sur les routes d'agent (`/api/ingest`,
`/api/agent/commands/*`, `/api/agent/relay*`). Ils disent qu'une machine
peut parler, pas *laquelle* : sur ces routes, une machine liée doit aussi
présenter son propre secret de liaison dans `X-DumbMonit-Agent-Secret`
(`dmab_…`), que le serveur donne une fois à l'enrôlement. Voir
[Liaison](../devices/agent.md#binding-one-machine-one-agent).

La connexion est limitée en débit : après cinq tentatives échouées, chaque
tentative suivante est refusée avec `429` et un délai `Retry-After` qui
double de 30 s jusqu'à 5 minutes. Ne devinez jamais des mots de passe en
boucle.

## Portées

Un jeton a l'une de deux portées. Il n'y en a pas de troisième, plus
puissante : ce qu'un administrateur fait sur son propre compte reste hors
de portée de chaque jeton.

| | Jeton `read` | Jeton `write` |
|---|---|---|
| Chaque `GET` qu'un compte *lecteur* peut faire (appareils, alertes, métriques, règles, canaux sans leurs secrets, pages d'état…) | oui | oui |
| `POST`, `PUT`, `DELETE` qu'un *administrateur* peut faire (appareils, règles, silences, canaux, pages d'état, packs, sondes, découverte…) | `403` | oui |
| `/metrics`, `/federate`, `/prometheus/*` ([sonder](metrics.md#scraping-dumbmonit)) | oui | oui |
| Outils MCP de lecture / d'écriture ([assistants](../using/assistant.md#what-the-assistant-can-call)) | lecture seule | les deux |
| Comptes, sessions, double authentification, SSO, journal d'audit (`/api/auth/*`, `/api/users/*`) | `403` | `403` |
| Jetons d'API et jetons d'enrôlement d'agent (`/api/tokens/*`, `/api/agent/tokens/*`) | `403` | `403` |
| Sauvegarde et restauration (`/api/backup/*`) | `403` | `403` |

Un jeton `read` sur une route d'écriture obtient `403` avec un message
nommant le jeton et la portée manquante. Comme aucun jeton ne peut lister,
créer ou révoquer des jetons, un jeton ne peut jamais en créer un autre —
encore moins un plus puissant.

## Sécurité des jetons

- **Montré une fois, stocké hashé.** Un jeton est `dmt_` suivi de 128 bits
  aléatoires. Le serveur ne garde que son empreinte SHA-256 (et les
  premiers caractères, `dmt_1a2b3c4d`, pour le reconnaître dans une liste) ;
  le jeton en clair n'existe que dans la réponse à sa création. Les jetons
  présentés sont comparés en temps constant.
- **Expiration.** Choisissez 30 jours, 90 jours, 1 an ou jamais à la
  création. Un jeton expiré obtient `401` avec
  `The API token "<name>" expired on …` et
  `WWW-Authenticate: Bearer error="invalid_token"` ; il reste dans la
  liste, marqué expiré, jusqu'à ce que vous le révoquiez.
- **Réseaux autorisés.** Restreignez optionnellement un jeton à une liste
  d'adresses ou de plages CIDR (`192.168.1.0/24, 10.8.0.5`, 32 entrées au
  plus). Depuis ailleurs il obtient `403` avec
  `The API token "<name>" cannot be used from <address>.` L'adresse
  vérifiée est celle du client TCP, ou — seulement quand la connexion
  vient d'un proxy listé dans `DUMBMONIT_TRUSTED_PROXIES` — celle que ce
  proxy rapporte dans `X-Forwarded-For`. Derrière un reverse proxy que
  vous n'avez pas déclaré, chaque requête semble venir du proxy :
  déclarez-le, ou la restriction refusera tout le monde.
- **Dernière utilisation.** La liste de jetons montre quand chaque jeton a
  été utilisé pour la dernière fois et depuis quelle adresse (mis à jour
  au plus une fois par minute, ou dès que l'adresse change) : un jeton que
  plus personne n'utilise est un à révoquer.
- **Lié à son créateur.** Un jeton se souvient du compte qui l'a créé. Si
  ce compte est désactivé, ses jetons sont refusés (`401`) ; s'il est
  rétrogradé en opérateur ou lecteur, ses jetons `write` agissent comme
  des jetons `read` ; s'il est supprimé, ses jetons sont révoqués.
- **La révocation** est immédiate : le prochain appel obtient `401`.
- **Journalisé.** Chaque écriture faite avec un jeton est écrite dans le
  journal du serveur avec le nom et l'id du jeton, la méthode, le chemin
  (sans la chaîne de requête) et le statut — jamais le jeton, jamais le
  corps. Créer et révoquer un jeton sont enregistrés dans le journal
  d'audit (Réglages → Sécurité).

## Limites de débit {#rate-limits}

Chaque jeton peut faire 120 appels par minute, REST et MCP ensemble.
Au-delà, les appels sont refusés avec `429` et un en-tête `Retry-After`
(secondes) jusqu'à la fin de la minute. Chaque réponse à une requête
authentifiée par jeton dit où vous en êtes :

| En-tête | Signification |
|---|---|
| `RateLimit-Limit` | Appels autorisés par fenêtre (120). |
| `RateLimit-Remaining` | Appels restants dans la fenêtre actuelle. |
| `RateLimit-Reset` | Secondes avant que la fenêtre ne redémarre. |

Un tableau de bord Grafana avec beaucoup de panneaux et un rafraîchissement
court peut atteindre la limite : donnez-lui son propre jeton, ou allongez
le rafraîchissement.

## Requêtes cross-origin (CORS) {#cross-origin-requests-cors}

Une page servie depuis une autre origine — votre propre tableau de bord,
Swagger UI, un client MCP basé navigateur — peut appeler l'API **avec un
jeton bearer, jamais avec une session** :

- une réponse porte `Access-Control-Allow-Origin` seulement quand la
  requête portait `Authorization: Bearer dmt_…` (ou pour le
  `GET /api/openapi.json` public). `Access-Control-Allow-Credentials`
  n'est jamais envoyé, un navigateur n'attache donc jamais le cookie de
  session à un appel cross-origin ;
- un preflight (`OPTIONS`) sur `/api/*` ne réussit que quand les en-têtes
  annoncés incluent `authorization` : `204`, méthodes
  `GET, POST, PUT, DELETE`, en-têtes
  `authorization, content-type, mcp-protocol-version, mcp-method, mcp-name`,
  mis en cache 10 minutes. `X-Requested-With` n'en fait pas partie, une
  session par cookie ne peut donc jamais faire une écriture depuis une
  autre origine ;
- `Retry-After`, `WWW-Authenticate`, les en-têtes `RateLimit-*` et
  `X-DumbMonit-Api-Version` sont lisibles par le script appelant.

`DUMBMONIT_API_CORS_ORIGINS` règle quelles origines sont autorisées :

| Valeur | Effet |
|---|---|
| non définie (défaut) | N'importe quelle origine (`Access-Control-Allow-Origin: *`). Sûr, puisque seul un jeton bearer ouvre quoi que ce soit. |
| `https://grafana.example.com,https://dash.lan` | Seulement ces origines, renvoyées en écho. |
| `off` | Aucun en-tête CORS du tout : seulement les pages de même origine et les clients non navigateur. |

La même liste décide quelle `Origin` le
[point de terminaison MCP](../using/assistant.md#security-notes) accepte.

```js
// Depuis une page sur une autre origine. N'envoyez jamais un jeton d'écriture dans une page publique.
const response = await fetch('https://monit.example.com/api/alerts', {
  headers: { Authorization: 'Bearer dmt_…' }
});
if (!response.ok) throw new Error((await response.json()).error);
const alerts = await response.json();
console.log(response.headers.get('RateLimit-Remaining'), 'appels restants cette minute');
```

## Versionnage {#versioning}

Chaque réponse sous `/api` porte la version d'API :

```
X-DumbMonit-Api-Version: 0.1
```

La même valeur est `info.version` dans le document OpenAPI, à côté de
`x-server-version`, la version du serveur qui a répondu. Tant que l'API
est en `0.x` elle n'est pas gelée ; la version change chaque fois qu'une
route ou un champ est supprimé ou renommé. Ajouter une route ou un champ
ne la change pas : ignorez les champs que vous ne connaissez pas.

## Erreurs

Chaque erreur est du JSON : `{"error": "message"}`, écrit pour une
personne.

| Statut | Quand |
|---|---|
| `400` | La requête est malformée ou une valeur est invalide ; le message la nomme. |
| `401` | Aucune session ni jeton valide : jeton manquant, inconnu, révoqué ou expiré (avec `WWW-Authenticate: Bearer`), ou un jeton dont le créateur est désactivé. |
| `403` | Authentifié, mais pas autorisé : un lecteur, un opérateur hors du traitement d'alerte, ou un jeton `read` sur une route d'écriture, un jeton sur une route de compte, de jeton ou de sauvegarde, un jeton utilisé depuis un réseau dont il n'est pas autorisé, une écriture par cookie sans `X-Requested-With`, ou toute écriture sur la démo publique (avec `"demo": true`). |
| `404` | L'id n'existe pas. Un chemin sous `/api` qui ne correspond à aucune route répond `404 {"error": "Unknown API route: …"}` plutôt que le HTML de l'UI web. |
| `405` | La méthode n'existe pas sur ce chemin (`GET /api/mcp`, par exemple). |
| `409` | Un conflit avec l'état actuel (un doublon, une commande déjà en cours…). |
| `429` | Limité en débit, avec `Retry-After`. |
| `500` | Une faute serveur ; les détails sont dans le journal serveur, jamais dans la réponse. |

```bash
$ curl -i -H 'Authorization: Bearer dmt_revoked…' http://monit.lan:8080/api/targets
HTTP/1.1 401 Unauthorized
www-authenticate: Bearer
{"error":"A valid API token is required."}

$ curl -i -X DELETE -H 'Authorization: Bearer dmt_…read…' http://monit.lan:8080/api/targets/4
HTTP/1.1 403 Forbidden
{"error":"This action needs a token with the \"write\" scope; the token \"Grafana\" is \"read\" only. Create a write token in Settings → API & assistants."}

$ curl -i -H 'Authorization: Bearer dmt_…' http://monit.lan:8080/api/alerts   # le 121e appel cette minute
HTTP/1.1 429 Too Many Requests
retry-after: 17
ratelimit-limit: 120
ratelimit-remaining: 0
{"error":"Rate limit reached (120 calls per minute per token). Retry in 17 s."}
```

Le [point de terminaison MCP](../using/assistant.md#protocol-details)
répond en JSON-RPC à la place, avec ses propres codes d'erreur.

## Comptes, connexion et jetons

Dans les tableaux ci-dessous, *session* signifie un cookie de session
**ou** un jeton d'API ; *admin* signifie une session administrateur ou un
jeton `write` ; *opérateur* signifie une session opérateur ou
administrateur, ou un jeton `write` ; *session seulement* signifie un
cookie de session, jamais un jeton.

**Rôles.** Un compte `viewer` fait chaque `GET`. Un `operator` fait les
mêmes lectures plus les écritures de traitement d'alerte marquées
*opérateur* sur cette page (et `x-roles: [admin, operator]` dans le
document OpenAPI) : acquitter, repousser (fenêtres de maintenance),
ignorer une règle pour un appareil, effacer l'historique résolu. Chaque
autre écriture est réservée aux administrateurs et répond `403` à un
opérateur.

| Méthode | Route | Auth | Objet |
|---|---|---|---|
| `GET` | `/api/auth/status` | public | `{"configured": bool, "authenticated": bool, "user": …, "oidc": {"enabled", "provider_name", "login_url"}}`. `configured: false` signifie une instance toute neuve : jusqu'à ce que le premier administrateur existe, chaque route marquée *session* répond `401` — seuls `status`, `setup`, `login` et `health` sont accessibles. |
| `POST` | `/api/auth/setup` | public | `{"setup_code": "…", "username": "admin", "password": "…"}`. Crée le premier administrateur sur une instance neuve (12 caractères au moins) ; `username` peut être omis et vaut alors `admin`. `setup_code` est le code à usage unique affiché dans les logs serveur au démarrage (ou `DUMBMONIT_SETUP_CODE`) ; casse, tirets et espaces sont ignorés. `204` ; n'ouvre pas de session. `401` pour un mauvais code, `429` après trop d'essais, `409` une fois qu'un compte existe. |
| `POST` | `/api/auth/login` | public | `{"username": "…", "password": "…"}`. `204` avec `Set-Cookie`. `401` sur un mauvais mot de passe, `429` en cas de limitation de débit. Quand le compte a la double authentification activée, `200` avec `{"totp_required": true, "pending": "…"}` à la place — un ticket qui vit cinq minutes et meurt après cinq mauvais codes : terminez avec `/api/auth/login/totp`. |
| `POST` | `/api/auth/login/totp` | public | `{"pending": "…", "code": "123456"}` — le ticket de la première étape et un code (ou un code de récupération). `204` avec le cookie de session. |
| `GET` | `/api/auth/oidc/start` | public | Redirige le navigateur vers le fournisseur d'identité. |
| `GET` | `/api/auth/oidc/callback` | public | Retour depuis le fournisseur (`code`, `state`) : ouvre la session et redirige vers l'UI. |
| `GET` | `/api/auth/me` | session seulement | Le compte actuel : `id`, `username`, `display_name`, `role` (`admin`, `operator`, `viewer`), `auth` (`password`, `oidc`), `disabled`, `totp_enabled`, `created_at`, `last_login_at`. |
| `POST` | `/api/auth/logout` | session seulement | Termine la session et efface le cookie. |
| `POST` | `/api/auth/password` | session seulement | `{"current_password": "…", "new_password": "…"}`. Déconnecte chaque autre session. |
| `GET` | `/api/auth/totp` | session seulement | État de double authentification du compte actuel : `enabled`, `pending`, `recovery_codes_left`. |
| `POST` | `/api/auth/totp/enroll` | session seulement | `{"password": "…"}`. Propose un secret : `secret` (base32), `otpauth_uri`, `issuer`, `account`. Rien n'est imposé avant vérification. |
| `POST` | `/api/auth/totp/verify` | session seulement | `{"code": "123456"}`. Confirme l'enrôlement et renvoie `{"recovery_codes": […]}` — montré une fois. |
| `DELETE` | `/api/auth/totp` | session seulement | `{"password": "…"}`. Retire le second facteur. `204`. |
| `GET` | `/api/auth/audit?limit=200` | admin, session seulement | Les derniers événements de sécurité : `id`, `at`, `actor`, `action` (`login`, `login.failed`, `password.changed`, `token.created`, `user.updated`, …), `subject`, `ip`. |
| `GET` | `/api/auth/oidc/config` | admin, session seulement | Les réglages SSO et d'où ils viennent (environnement ou base de données) ; le secret client n'est jamais renvoyé. |
| `PUT` | `/api/auth/oidc/config` | admin, session seulement | `{"issuer", "client_id", "client_secret", "provider_name", "scopes", "auto_create", "admin_groups", "groups_claim", "public_url"}`. Un `client_secret` absent ou vide garde celui enregistré ; un `auto_create` absent signifie désactivé. |
| `DELETE` | `/api/auth/oidc/config` | admin, session seulement | Oublie les réglages SSO enregistrés (les variables d'environnement, s'il y en a, s'appliquent à nouveau). `204`. |
| `POST` | `/api/auth/oidc/test` | admin, session seulement | Récupère le document de découverte du fournisseur et rapporte ce qu'il a trouvé. |
| `GET` | `/api/users` | admin, session seulement | Chaque compte, dans la forme de `/api/auth/me`. |
| `POST` | `/api/users` | admin, session seulement | `{"username", "display_name", "role", "password"}`. `password` peut être omis seulement quand SSO est activé. `201`. |
| `PUT` | `/api/users/{id}` | admin, session seulement | N'importe lequel de `display_name`, `role`, `disabled`, `password` ; un champ omis garde sa valeur. Ne peut pas rétrograder ou désactiver le dernier administrateur. |
| `DELETE` | `/api/users/{id}` | admin, session seulement | `204`. Pas vous-même, pas le dernier administrateur. |
| `DELETE` | `/api/users/{id}/totp` | admin, session seulement | Réinitialise le second facteur d'un autre compte (un collègue bloqué). `204`. |
| `GET` | `/api/tokens` | session seulement | Chaque jeton d'API : `id`, `name`, `prefix`, `scope`, `created_at`, `created_by` (le compte qui l'a créé), `expires_at` (`null` : jamais), `expired`, `allowed_networks` (vide : n'importe où), `last_used_at`, `last_used_ip`, `revoked_at`. Jamais le secret. |
| `POST` | `/api/tokens` | admin, session seulement | `{"name": "Grafana", "scope": "read", "expires_in_days": 90, "allowed_networks": ["192.168.1.0/24"]}`. `name` est requis (80 caractères au plus) ; `scope` est `read` (défaut) ou `write` ; `expires_in_days` va de 1 à 3650, `null` ou absent pour un jeton qui n'expire jamais ; `allowed_networks` prend jusqu'à 32 adresses ou plages CIDR, stockées sous forme de réseau (`192.168.1.7/24` devient `192.168.1.0/24`), vide ou absent pour n'importe où. `201` avec les champs du jeton plus `secret` (montré une fois). `400` nomme un réseau invalide. |
| `DELETE` | `/api/tokens/{id}` | admin, session seulement | Révoquer. `204` ; `404` quand inconnu ou déjà révoqué. |

Un jeton tel que listé :

```json
{
  "id": 3, "name": "Grafana", "prefix": "dmt_1a2b3c4d", "scope": "read",
  "created_at": "2026-10-01T09:12:00Z", "created_by": "admin",
  "expires_at": "2026-12-30T09:12:00Z", "expired": false,
  "allowed_networks": ["192.168.1.0/24"],
  "last_used_at": "2026-10-01T09:30:00Z", "last_used_ip": "192.168.1.20",
  "revoked_at": null
}
```

## En-têtes de réponse

En plus de `X-DumbMonit-Api-Version` ([versionnage](#versioning)), des
en-têtes `RateLimit-*` ([limites de débit](#rate-limits)) et des en-têtes
CORS ([requêtes cross-origin](#cross-origin-requests-cors)), chaque réponse
porte `X-Content-Type-Options: nosniff`, `Referrer-Policy: same-origin` et
une `Content-Security-Policy`. La politique part de `default-src 'none'`
et n'ouvre que ce que l'interface utilise réellement — tout servi par
DumbMonit lui-même :

```
default-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:;
font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self';
script-src 'self' 'nonce-<per response>'; frame-ancestors 'none'
```

Il n'y a aucune origine tierce dedans, et il ne devrait jamais y en avoir :
les polices, la bibliothèque de graphiques et chaque ressource sont
fournies dans le binaire. Les scripts en ligne de la page (le thème
appliqué avant le premier rendu, le démarrage de SvelteKit) sont autorisés
par un nonce régénéré pour chaque réponse, `'unsafe-inline'` ne s'applique
donc jamais aux scripts. Il s'applique aux styles, et ne peut pas être
retiré : Svelte règle des attributs `style=` sur des éléments, que ni un
nonce ni un hash ne peuvent couvrir.

Les pages d'état publiques sont faites pour être intégrées : `/s/<slug>`
et sa vue compacte `/s/<slug>/embed` — et seulement ces deux — obtiennent
la même politique sans `frame-ancestors`. Tout le reste, les pages de
confirmation d'abonnement et de désabonnement sous `/s/` incluses, porte
aussi `X-Frame-Options: DENY`.

Si vous mettez DumbMonit derrière un reverse proxy, ne le laissez pas
ajouter un second en-tête `Content-Security-Policy` : les navigateurs
imposent l'intersection de tous, et un défaut de proxy sans le nonce laisse
une page blanche.

## Santé

| Méthode | Route | Auth | Objet |
|---|---|---|---|
| `GET` | `/api/health` | public | Version du serveur et l'état de ses dépendances. Toujours `200` : un composant en échec est dans le corps. |
| `GET` | `/api/update` | session ou jeton | Version en cours, dernière release, `update_available` ; le serveur demande à GitHub une fois par jour au plus. `POST /api/update/check` regarde maintenant, `PUT /api/update/settings` active ou désactive la vérification. Voir [Mise à jour](../install/update.md). |

```json
{"status":"ok","version":"0.1.0","database":{"ok":true},"victoria":{"ok":true,"embedded":true}}
```

`status` est `degraded` quand un composant échoue ; ce composant porte
alors une chaîne `error`. `victoria.embedded` est `true` quand le serveur
fait tourner son propre VictoriaMetrics, `false` quand `DUMBMONIT_VM_URL`
pointe vers un externe.

## Sauvegarde et restauration

| Méthode | Route | Auth | Objet |
|---|---|---|---|
| `GET` | `/api/backup` | admin, session seulement | Ce qu'un paquet contiendrait (`contents`, une ligne par section avec son compteur), la version du format de paquet, où vit le secret de l'instance (`secret_source` : `file` ou `environment`) et l'état des sauvegardes locales planifiées (`schedule`). |
| `POST` | `/api/backup` | admin, session seulement | `{"passphrase": "…", "include_account_secrets": false}`. Renvoie le paquet lui-même comme un fichier JSON : un en-tête en clair (`format`, `version`, `created_at`, `summary`, `kdf`) et un `payload` chiffré avec AES-256-GCM sous une clé dérivée de la phrase de passe par Argon2id. 16 caractères au moins, sinon `400`. |
| `POST` | `/api/backup/restore` | admin, session seulement | `{"bundle": {…}, "passphrase": "…", "apply": false}`. Essai à sec sauf si `apply` est `true` ; les deux lancent le même code, l'essai à sec dans une transaction qui est annulée. Répond le rapport : par section, `created`, `updated`, `skipped` et `notes`, plus `warnings`. `400` sur une phrase de passe incorrecte, un fichier modifié, ou un paquet écrit dans une version de format plus récente. |
| `POST` | `/api/backup/local` | admin, session seulement | Écrit une sauvegarde locale de style planifiée tout de suite et répond le nouveau `schedule`. |

Un paquet porte chaque identifiant de l'instance, rechiffré avec la phrase
de passe, ces routes refusent donc les jetons d'API avec une explication :
seul un administrateur connecté à l'interface web en obtient un. Voir
[Sauvegarde et restauration](../install/backup.md).

## Types d'appareil

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/collectors` | Chaque type d'appareil que ce serveur peut surveiller : `kind`, `label`, `summary`, `examples`, `credential_types`, `address_hint`, `default_port`, `setup` (`title`, `steps`, `warning`, `doc_url`) et `options` (`key`, `label`, `help`, `placeholder`, `default`, `required`, `input`, `choices`). Les types apportés par un [pack d'intégration](../packs/index.md) sont listés de la même façon, comme `pack.<id>`. |

## Packs d'intégration

Session admin ou jeton d'API (portée `write` pour les changements). Voir
[Packs d'intégration](../packs/index.md).

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/packs` | Packs installés : `id`, `version`, `label`, `summary`, `kind` (`null` pour un pack avec seulement des profils SNMP), `enabled`, `installed_at`, `sha256`, `metrics`, `rules` (uids), `snmp_profiles`, `targets` (appareils qui l'utilisent), `warnings`, et `error` quand le pack enregistré ne passe plus la validation. |
| `GET` | `/api/packs/{id}` | Un pack. |
| `POST` | `/api/packs` | Installer ou mettre à jour. Corps : le YAML (`application/yaml`, `text/plain`…) ou `{"yaml": "…"}` en JSON. `201` quand créé, `200` quand mis à jour ou inchangé, avec `outcome` (`created`, `updated`, `unchanged`), `pack`, `rules_added` et `restart_required` (profils SNMP). `400` liste chaque erreur de validation. |
| `PUT` | `/api/packs/{id}/enable`, `/api/packs/{id}/disable` | Enregistrer ou retirer le type d'appareil du pack ; appareils, historique et règles restent. |
| `DELETE` | `/api/packs/{id}` | Désinstaller, avec ses règles. `409` tant que des appareils l'utilisent. |

## Targets (appareils)

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/targets` | Liste chaque appareil. |
| `POST` | `/api/targets` | En créer un. `201` avec l'appareil. La détection de profil démarre en arrière-plan pour SNMP. |
| `POST` | `/api/targets/import/kuma` | Importe des moniteurs depuis une sauvegarde d'Uptime Kuma 1.x (le fichier JSON en corps, 4 Mio au plus). `?dry_run=true` pour un aperçu. Voir [Services](../devices/services.md#venir-duptime-kuma). |
| `GET` | `/api/targets/{id}` | Un appareil. |
| `PUT` | `/api/targets/{id}` | Le remplacer. Omettre `credential` garde celui enregistré ; omettre `profile_id` garde le profil détecté (`""` l'efface). Régler `enabled: false` efface les alertes de l'appareil sans notifier. |
| `DELETE` | `/api/targets/{id}` | `204`. Efface les alertes de l'appareil sans notifier et supprime ses séries temporelles de VictoriaMetrics (au mieux). |
| `POST` | `/api/targets/{id}/probe` | Sonder maintenant : `{"sample_count": 42, "series": ["if_octets_in{host=\"Core switch\",ifname=\"eth0\"}", …]}` — les clés de série comme le collecteur les a produites, sans le préfixe `dumbmonit_` que l'écrivain ajoute en chemin vers VictoriaMetrics. |
| `POST` | `/api/targets/{id}/discover` | Relancer la détection de profil : `{"profile_id": "host-resources"}` ou `null`. |

Un appareil tel que renvoyé :

```json
{
  "id": 4,
  "name": "Core switch",
  "address": "192.168.1.2",
  "kind": "snmp",
  "profile_id": "host-resources",
  "parent_id": null,
  "via_agent": null,
  "interval_secs": 60,
  "enabled": true,
  "tags": {"role": "network"},
  "credential_kind": "SNMP community",
  "last_probe_at": "2026-09-15 13:20:05",
  "last_error": null,
  "error_kind": null
}
```

Le secret lui-même n'est jamais renvoyé : `credential_kind` est un label.
Les horodatages serveur sont en UTC sans suffixe. `error_kind` classe
`last_error` : `down` (l'appareil n'a pas répondu) ou `config` (de notre
côté — identifiants, adresse, option), ce que l'UI montre comme
*Inaccessible* ou *Mal configuré* ; `null` quand la dernière sonde a
réussi.

Le créer :

```json
{
  "name": "Core switch",
  "address": "192.168.1.10",
  "kind": "snmp",
  "interval_secs": 60,
  "parent_id": null,
  "tags": {},
  "credential": {"type": "snmp_community", "community": "public"}
}
```

Formes d'identifiant (`type`) : `none` ; `snmp_community` (`community`) ;
`snmp_v3` (`username`, `auth: {protocol, passphrase}` optionnel avec
`md5`, `sha1`, `sha224`, `sha256`, `sha384` ou `sha512`, `privacy:
{protocol, passphrase}` optionnel avec `des`, `aes128`, `aes192` ou
`aes256`, `context` optionnel) ; `api_token` (`token`, ou les deux moitiés
`token_id` et `secret`, que le serveur joint en `user@realm!name=secret`
pour Proxmox VE et PBS) ; `username_password` (`username`, `password`).
`interval_secs` vaut 60 par défaut et ne peut pas descendre sous 10.
`name` fait 200 caractères au plus et `address` 253 au plus (`400`
au-delà). `parent_id` doit nommer un appareil existant
(`400 Parent device N not found.`). Les options de type vont dans `tags`
sous leur clé.

`via_agent` nomme l'[agent relais](../install/remote-site.md) qui sonde
l'appareil depuis son propre réseau plutôt que le serveur ; `null` (le
défaut) signifie que le serveur le sonde. Il doit être un appareil
`agent` (`400` sinon), un appareil ne peut pas se relayer lui-même, et un
appareil `agent` ne peut pas être relayé. Sur `PUT`, un `via_agent`
*omis* garde la valeur enregistrée, comme `profile_id` et `credential` ;
envoyez `null` pour revenir à la sonde directe. `POST
/api/targets/{id}/probe` et `/discover` sur un appareil relayé passent
par le relais et attendent sa réponse (`409` quand une sonde est déjà en
vol, `400 Timed out: relay agent …` quand il ne répond pas).

## Découverte

| Méthode | Route | Auth | Objet |
|---|---|---|---|
| `POST` | `/api/discovery` | admin | `{"cidr": "192.168.1.0/24", "community": "public", "port": 161, "timeout_ms": 1000}`. Scanne un réseau pour des appareils SNMP. `cidr` est requis ; `community` vaut `public` par défaut ; `timeout_ms` est borné à 100–10 000. Les réseaux de plus de 4096 adresses sont refusés. Le scan est un `POST` avec un corps JSON pour que la communauté n'apparaisse jamais dans une URL ou une ligne de log. |

```json
{
  "scanned": 254,
  "devices": [
    {"address": "192.168.1.10", "sysname": "sw-core", "sysdescr": "…", "sysobjectid": "1.3.6.1.4.1.8072.3.2.10", "suggested_profile": "host-resources"}
  ]
}
```

## Guide de premier démarrage

Les trois étapes que la vue d'ensemble montre sur une instance neuve.
L'état est gardé sur l'instance, pas dans le navigateur, passer une étape
la passe donc partout.

| Méthode | Route | Auth | Objet |
|---|---|---|---|
| `GET` | `/api/onboarding` | session | Où en est l'instance. |
| `PUT` | `/api/onboarding` | admin | `{"skipped": true}` range tout le guide ; `{"dismissed": ["test"]}` range des étapes individuelles. Un champ omis garde sa valeur enregistrée ; un id d'étape autre que `device`, `channel` ou `test` est un `400`. Renvoie le même document que `GET`. |

```json
{
  "skipped": false,
  "complete": false,
  "completed_at": null,
  "dismissed": [],
  "has_target": true,
  "has_channel": true,
  "notification_confirmed": false
}
```

`has_target`, `has_channel` et `notification_confirmed` sont recalculés à
chaque lecture : ce sont des faits sur l'instance, pas des drapeaux que
l'interface règle. `notification_confirmed` est vrai une fois qu'un
message a vraiment quitté un canal — un message de test compte, un canal
juste créé non.

`complete` est verrouillé et ne revient jamais à false. Le serveur le
règle quand les trois étapes sont réglées (faites, ou passées), et aussi à
la toute première lecture d'une instance qui avait déjà un appareil et un
canal — mettre à jour une instance établie ne lui montre donc jamais un
guide de premier démarrage.

## Métriques {#metrics}

Les deux routes relaient VictoriaMetrics et renvoient des séries dans la
forme Prometheus :
`[{"metric": {labels…}, "values": [[timestamp_seconds, "value"], …]}]`.

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/metrics/query?query=…` | Requête MetricsQL instantanée. |
| `GET` | `/api/metrics/query_range?query=…&start=…&end=…&step=…` | Requête de plage. `start` et `end` sont en **millisecondes** depuis l'epoch ; `step` est en secondes et est élargi pour qu'une série ne dépasse jamais 2 000 points. |

```bash
curl -b cookies.txt -G http://localhost:8080/api/metrics/query \
  --data-urlencode 'query=avg by (target, host) (dumbmonit_cpu_usage_percent)'
```

```json
[{"metric":{"host":"ThinkpadE14","target":"3"},"values":[[1757941205,"12.5"]]}]
```

## Sondage (hors `/api`)

Pour un Prometheus ou un Grafana existant. Les deux prennent un jeton
d'API `read` dans `Authorization: Bearer dmt_…` (un cookie de session
fonctionne aussi) ; un scraper n'envoie aucun en-tête `X-Requested-With`
et n'en a besoin d'aucun — ces routes ne font que lire. Voir la
[référence des métriques](metrics.md#scraping-dumbmonit) pour un
`scrape_config` prêt à copier et la source de données Grafana.

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/metrics` | Santé de l'instance elle-même au format texte Prometheus : planificateur, sondes, écritures, alertes, notifications, agents, base de données, VictoriaMetrics. Quelques dizaines de séries, jamais une par appareil. |
| `GET` | `/federate` | Les mesures, dans le même format. `match[]` sélectionne des séries (répétable, 10 au plus, `{__name__=~"dumbmonit_.*"}` par défaut), `max_lookback` élargit la fenêtre dans laquelle le dernier point est cherché (`5m` par défaut). `400` au-delà de 8 Mio, avec quoi restreindre. |
| `GET`, `POST` | `/prometheus/api/v1/{route}` | La moitié lecture de l'API Prometheus, relayée vers le magasin, pour que Grafana puisse utiliser `http://…/prometheus` comme source de données Prometheus. Seulement `query`, `query_range`, `query_exemplars`, `series`, `labels`, `label/{name}/values`, `metadata` et `status/buildinfo` ; tout le reste est `404`. |

`DUMBMONIT_METRICS_PUBLIC=true` sert les trois sans jeton — seulement
derrière un pare-feu, puisqu'elles portent chaque mesure de chaque
appareil.

## Alertes

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/alerts` | Alertes actives (en construction, actives, supprimées, résolues récemment). Les alertes d'appareils supprimés ou en pause ne sont jamais listées. |
| `GET` | `/api/alerts/history?since=2026-09-01T00:00:00Z&limit=200&dismissed=false` | Transitions de phase. `since` est en RFC 3339, défaut les sept derniers jours ; `limit` doit être positif. `dismissed` (défaut `false`) exclut les entrées effacées avec les deux routes ci-dessous ; rien n'est jamais détruit par un effacement, seulement caché de la vue par défaut. |
| `POST` | `/api/alerts/history/{id}/dismiss` | Efface une entrée `resolved` de la vue d'historique par défaut. `204`. `404` sur un id inconnu ou une transition encore active. Opérateur ; audité. |
| `POST` | `/api/alerts/history/dismiss-resolved` | Efface chaque entrée `resolved` pas encore effacée d'un coup (« Effacer tout ce qui est résolu »). Renvoie `{"dismissed": <count>}`. Opérateur ; audité. |
| `POST` | `/api/alerts/{fingerprint}/ack` | Acquitter : `{"duration_secs": 14400, "note": "…"}` ou `{"until": "2026-09-22T18:00:00Z"}` (l'un des deux ; aucun des deux signifie 4 heures, 30 jours au plus). `{"until": null}` le lève. Renvoie l'alerte. Opérateur ; audité. |
| `DELETE` | `/api/alerts/{fingerprint}/ack` | Lever l'acquittement. Renvoie l'alerte. Opérateur. |

Une alerte active :

```json
{
  "fingerprint": "…",
  "rule_uid": "disk_almost_full",
  "rule_name": "Disk almost full",
  "severity": "warning",
  "target_id": 4,
  "series_key": "…",
  "labels": {"host": "Core switch", "mountpoint": "/", "target": "4"},
  "phase": "firing",
  "effective_phase": "suppressed",
  "suppressed": true,
  "suppressed_by": 2,
  "silenced": false,
  "learning": false,
  "acked": true,
  "acked_until": "2026-09-15T17:20:00Z",
  "acked_by": "admin",
  "ack_note": "replacing the disk",
  "value": 95.96,
  "score": null,
  "condition_since": "2026-09-15T12:00:00Z",
  "firing_since": "2026-09-15T12:15:00Z",
  "last_eval_at": "2026-09-15T13:20:00Z",
  "last_notified_at": null,
  "notify_count": 0
}
```

Une entrée d'historique a `id`, `fingerprint`, `rule_uid`, `target_id`,
`from_phase`, `to_phase`, `severity`, `value`, `notified`, `reason` (vide,
ou pourquoi rien n'a été envoyé : `learning: would have fired`,
`suppressed: device 2 unreachable`, `maintenance window`,
`acknowledged by admin`, `device removed or disabled`), `at` et
`dismissed` (effacée de la vue active ; la ligne elle-même est gardée).
`acked` est vrai tant que `acked_until` est dans le futur : rappels et
escalades se mettent en pause, la résolution est quand même notifiée et
efface l'acquittement. Un `target_id` qui n'existe plus est montré comme
« (appareil supprimé) » par l'UI.

### Règles

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/alerts/rules` | Chaque règle. |
| `POST` | `/api/alerts/rules` | Créer. `201`. |
| `PUT` | `/api/alerts/rules/{id}` | Mettre à jour. Une règle intégrée garde sa requête. |
| `DELETE` | `/api/alerts/rules/{id}` | `204`. Les règles intégrées ne peuvent pas être supprimées. |
| `POST` | `/api/alerts/rules/{id}/enable` | `{"enabled": false}`. Renvoie la règle. |

Une règle :

```json
{
  "id": 2, "uid": "cpu_high", "name": "High CPU",
  "description": "CPU load sustained above 90%.",
  "kind": "threshold",
  "query": "avg by (target, host) (dumbmonit_cpu_load_percent or dumbmonit_proxmox_node_cpu_percent or dumbmonit_cpu_usage_percent)",
  "operator": ">", "threshold": 90, "for_secs": 600,
  "severity": "warning",
  "selector": {"kind": "all"},
  "channels": [],
  "params": {"k": 3.5, "alpha": 0.05, "mad_floor_abs": 1e-6, "mad_floor_rel": 0.01, "min_samples": 3},
  "unit": "%", "repeat_secs": 21600, "escalate_after_secs": 3600,
  "enabled": true, "builtin": true
}
```

Le créer demande au moins `name` et `query` ; `kind` (`threshold`,
`anomaly`, `predict`), `operator`, `threshold`, `for_secs`, `severity`
(`info`, `warning`, `critical`), `selector` (`{"kind":"all"}`,
`{"kind":"ids","ids":[…]}` ou `{"kind":"labels","labels":{…}}`), `channels`
(ids de canal ; vide signifie chaque canal activé), `unit`, `repeat_secs`,
`escalate_after_secs` et `enabled` sont optionnels.

### Silences (fenêtres de maintenance)

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/alerts/silences` | Chaque fenêtre, avec `active_now`, `active_until` et `next_start_at` (RFC 3339, calculé par le serveur). |
| `POST` | `/api/alerts/silences` | Créer. `201`. Opérateur. |
| `DELETE` | `/api/alerts/silences/{id}` | `204`. Opérateur. |

```json
{
  "name": "Sunday backups",
  "comment": "NAS is busy",
  "target_id": 4,
  "matchers": {},
  "schedule": {"kind": "weekly", "days": [6], "start_minute": 120, "end_minute": 240, "timezone": "Europe/Paris"},
  "enabled": true
}
```

Un calendrier ponctuel est
`{"kind": "once", "starts_at": "2026-09-20T22:00:00Z",
"ends_at": "2026-09-21T02:00:00Z"}`. Un mensuel est
`{"kind": "monthly", "days": [1], "nth_weekdays": [{"nth": 1, "weekday": 6}],
"start_minute": 120, "duration_minutes": 120, "timezone": "Europe/Paris"}` —
`nth` est 1 à 5, ou `-1` pour le dernier du mois ; un mois sans cette
occurrence est sauté. Les jours sont 0 = lundi … 6 = dimanche (1–31 pour
un `days` mensuel) ; les minutes sont depuis minuit local (0–1439). Sur un
calendrier récurrent, `timezone` (un nom IANA) gagne sur
`utc_offset_minutes` et c'est ce qui garde une fenêtre à son heure locale
à travers les changements d'heure d'été ; un nom de fuseau inconnu est
refusé.

### Surcharges par appareil {#per-device-overrides}

Une règle peut être ajustée pour un appareil sans toucher la règle
elle-même. `{"enabled": false}` est ce que l'action **Ignorer** de l'UI
(sur l'alerte, la page d'appareil ou la règle) écrit : l'appareil est
retiré de cette seule règle pour de bon, réversiblement, sans désactiver
la règle pour personne d'autre.

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/alerts/overrides?target_id=4` | Chaque surcharge, ou celles d'un appareil : `rule_uid`, `target_id`, `threshold`, `clear_threshold`, `enabled`. |
| `GET` | `/api/alerts/rules/{id}/overrides` | Les surcharges d'une règle. |
| `PUT` | `/api/alerts/rules/{id}/overrides/{target_id}` | `{"threshold": 95, "clear_threshold": 90, "enabled": true}` — chaque champ optionnel ; `null` signifie « comme la règle ». Crée ou remplace. Un opérateur ne peut envoyer que `{"enabled": false}` (ignorer) ; les surcharges de seuil de l'appareil, s'il y en a, sont gardées. |
| `DELETE` | `/api/alerts/rules/{id}/overrides/{target_id}` | `204`. Pour un opérateur, ne lève que l'ignorer : les surcharges de seuil restent. |
| `GET` | `/api/targets/{id}/rules` | Les règles qui surveillent un appareil : celles dont la requête lit une métrique qu'il a écrite ces dernières 24 h. `{"status": "ok" \| "no_data" \| "unavailable", "rules": [{id, uid, name, description, severity, enabled, ignored, custom_threshold, firing}]}`. |

## Politique de notification

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/notify/policy` | La politique globale : `batch_window_secs` (0 = envoyer tout de suite), `max_per_hour` par canal (0 = illimité), `flap_events`, `flap_window_secs`, `flap_hold_secs` (0 = pas de détection de flapping), `public_url` (liens dans les messages ; vide = `DUMBMONIT_PUBLIC_URL`), `escalate_after_secs` et `escalate_channel` (0 / `null` = pas d'escalade). |
| `PUT` | `/api/notify/policy` | Mêmes champs, chacun optionnel : un champ omis garde sa valeur. `"escalate_channel": null` désactive l'escalade. Renvoie la politique. |
| `POST` | `/api/notify/match-preview` | `{"matcher": {…}}` → quels appareils un filtre de routage de canal sélectionne : `devices` (`id`, `name`, `kind`, `tags`, `matched`), `matched`, `total`, `rules`, `excluded_rules`. |

## Canaux de notification

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/notify/kinds` | Chaque type de canal avec ses champs `settings` et `secrets` (`key`, `label`, `required`, `input`, `help`, `placeholder`, `options`, `shape`, `default`) et un `doc_url` comme `https://dumbmonit.readthedocs.io/en/latest/notifications/#discord`. |
| `GET` | `/api/notify/channels` | Chaque canal : `id`, `name`, `kind`, `enabled`, `settings`, `has_secret`, `last_error`, `last_sent_at`, `policy` (`min_severity`, `notify_resolved`, `min_interval_secs`, `quiet_hours`, `matcher`). Les secrets ne sont jamais renvoyés. |
| `POST` | `/api/notify/channels` | Créer. `201`. |
| `PUT` | `/api/notify/channels/{id}` | Mettre à jour. Omettre `secrets` garde ceux enregistrés ; `"secrets": {}` les efface. Dans `policy`, omettre `matcher` garde le filtre de routage et `"matcher": null` l'efface. |
| `DELETE` | `/api/notify/channels/{id}` | `204`. |
| `POST` | `/api/notify/channels/{id}/test` | Envoyer un message de test : `{"ok": true, "message": "…"}`. |

```json
{
  "name": "Ops Discord",
  "kind": "discord",
  "enabled": true,
  "settings": {"username": "DumbMonit"},
  "secrets": {"webhook_url": "https://discord.com/api/webhooks/…"}
}
```

Les clés exactes par type viennent de `/api/notify/kinds` et sont
documentées dans [Canaux de notification](../notifications.md).

### Appareils Web Push

Les appareils du **compte connecté** (session seulement, lecteurs
inclus ; les jetons d'API obtiennent `401`). Un canal `webpush` leur
envoie des messages ; voir
[Notifications push (PWA)](../using/push-notifications.md).

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/webpush` | `{"public_key": "…", "devices": […]}` : la clé publique VAPID (base64url, l'`applicationServerKey` pour s'abonner) et les appareils du compte — `id`, `device` (`Firefox on Android`), `push_service` (hôte), `fingerprint` (16 premiers caractères hexadécimaux du SHA-256 du endpoint), `created_at`, `last_success_at`, `last_error`. L'endpoint, les clés navigateur et la clé privée ne sont jamais renvoyés. |
| `POST` | `/api/webpush/subscriptions` | `PushSubscription.toJSON()` : `{"endpoint": "https://…", "keys": {"p256dh": "…", "auth": "…"}}`. `201` avec l'appareil. L'endpoint doit être en HTTPS sur une adresse publique : les adresses privées, loopback et link-local obtiennent `400`. Le même endpoint à nouveau remplace sa ligne. |
| `DELETE` | `/api/webpush/subscriptions/{id}` | `204` ; `404` pour un appareil d'un autre compte. |
| `POST` | `/api/webpush/test` | `{}` ou `{"id": 3}`. Envoie une notification de test : `{"ok", "delivered", "failed", "removed", "message"}`. Les abonnements répondus par `404`/`410` sont supprimés. |

## Rapports par email {#email-reports}

Administrateurs seulement, `GET` inclus (la réponse liste les
destinataires). Les jetons bearer ont besoin de la portée `write` pour les
changements. Voir [Rapports par email](../using/reports.md).

| Méthode | Route | Auth | Objet |
|---|---|---|---|
| `GET` | `/api/reports/schedules` | admin | Chaque rapport : `id`, `name`, `enabled`, `frequency`, `weekday` (0 est lundi), `day_of_month`, `hour`, `timezone`, `recipients`, `channel_id`, `last_sent_at`, `last_error`, `next_run_at`. |
| `POST` | `/api/reports/schedules` | admin | Les mêmes champs modifiables ; chacun vaut par défaut hebdomadaire, lundi, 8h00, `UTC`. `recipients` contient 20 adresses au plus et est requis pour activer le rapport ; `channel_id` doit être un canal `smtp` ou `null`. `201` ; `409` au-delà de 10 rapports. |
| `GET` | `/api/reports/schedules/{id}` | admin | Un rapport. |
| `PUT` | `/api/reports/schedules/{id}` | admin | Remplacement complet. |
| `DELETE` | `/api/reports/schedules/{id}` | admin | `204`. |
| `POST` | `/api/reports/schedules/{id}/send-test` | admin | Envoie un aperçu aux destinataires enregistrés : `{"sent": 2, "failed": 0}`. Ne touche pas le calendrier. `400` sans canal email utilisable ; `409` dans les 30 secondes de l'aperçu précédent. |

## Pages d'état et incidents

| Méthode | Route | Auth | Objet |
|---|---|---|---|
| `GET` | `/api/status-pages` | session | Chaque page avec ses éléments : `page` (`id`, `slug`, `title`, `description`, `published`, `theme`, `show_uptime_days`, `created_at`, `updated_at`) et `items` (`id`, `page_id`, `target_id`, `label`, `group_name`, `position`). |
| `POST` | `/api/status-pages` | admin | `{"title", "slug", "description", "published", "theme", "show_uptime_days"}`. `title` est requis (120 caractères au plus) ; `slug` (`^[a-z0-9-]{2,40}$`) est dérivé du titre quand omis ; `theme` est `auto` (défaut), `light` ou `dark` ; `show_uptime_days` va de 7 à 90 (90 par défaut). `201` ; `409` sur un slug déjà pris. |
| `GET` | `/api/status-pages/{id}` | session | Une page avec ses éléments. |
| `PUT` | `/api/status-pages/{id}` | admin | Mêmes champs, mais un remplacement complet plutôt qu'un patch : `title` est requis et chaque champ omis revient à son défaut (slug redérivé du titre, description vide, `published: false`, `theme: auto`, 90 jours de disponibilité). |
| `DELETE` | `/api/status-pages/{id}` | admin | `204`. L'URL publique arrête de répondre. |
| `POST`, `PUT` | `/api/status-pages`, `/api/status-pages/{id}` — apparence et abonnements | admin | Acceptent aussi `accent` (`default`, `blue`, `teal`, `violet`, `rose`, `amber`), `scenes` (ids de scène de la liste de l'éditeur — villes, nouvelles merveilles et merveilles antiques, p. ex. `venice`, `paris`, `giza`, `tajmahal`, sans doublons ; `[]` pour aucune ; omis à la création : `["paris"]`), `simple` (booléen, défaut `false` : la page publique ne montre aucune scène, mascotte ni animation et suit la préférence clair/sombre du visiteur), `scene_rotation` (`visit`, `1m`, `10m` ou `1h`), `footer_text` (texte brut, 280 caractères), `homepage_url` (`http(s)://` ou vide), `subscribe_channel_id` (un canal `smtp`, ou `null` pour désactiver l'abonnement email) et `domain` (le nom d'hôte public de la page, comme `status.example.com`, ou `null`/`""` pour aucun — voir [Domaine personnalisé](../using/status-pages.md#custom-domain)). `domain` est mis en minuscules et perd un préfixe `https://` ou une barre oblique finale ; un port, un chemin ou une adresse IP sont `400`, l'adresse de DumbMonit lui-même (son URL publique, ou l'hôte sur lequel la requête est arrivée) est `400`, et un nom qu'une autre page utilise est `409`. À la différence des autres champs, ces huit gardent leur valeur enregistrée quand omis d'un `PUT`. La vue de page les renvoie avec `logo_type` (`image/png`, `image/jpeg`, `image/webp` ou `null`). |
| `GET`, `PUT`, `DELETE` | `/api/status-pages/{id}/logo` | session / admin | Le logo de la page. `PUT` prend `{"data": "<base64 ou URL data:>"}` : PNG, JPEG ou WebP reconnu par son contenu, 256 Kio au plus, autre chose `400`. `204`. `GET` est l'aperçu de l'éditeur (brouillons inclus). |
| `GET` | `/api/status-pages/{id}/subscribers` | session | Abonnés email : `id`, `page_id`, `email`, `confirmed_at` (`null` tant qu'en attente), `created_at`. Leurs jetons ne quittent jamais le serveur. |
| `DELETE` | `/api/status-pages/{id}/subscribers/{subscriber}` | admin | Retire une adresse. `204`. |
| `PUT` | `/api/status-pages/{id}/items` | admin | `[{"target_id": 4, "label": "NAS", "group_name": "Storage"}, …]` — la liste complète et ordonnée des appareils montrés sur la page. |
| `GET` | `/api/incidents` | session | Chaque incident avec ses mises à jour : `incident` (`id`, `page_id`, `title`, `kind`, `status`, `severity`, `starts_at`, `ends_at`, `created_at`, `updated_at`) et `updates`. |
| `POST` | `/api/incidents` | admin | `{"title", "kind", "status", "severity", "page_id", "starts_at", "ends_at", "body"}` — `body` est la première mise à jour. `201`. |
| `PUT` | `/api/incidents/{id}` | admin | Mêmes champs ; un champ omis garde sa valeur. |
| `DELETE` | `/api/incidents/{id}` | admin | `204`. |
| `GET` | `/api/incidents/{id}/updates` | session | La chronologie : `id`, `incident_id`, `status`, `body`, `created_at`. |
| `POST` | `/api/incidents/{id}/updates` | admin | `{"status": "monitoring", "body": "…"}`. Ajoute une mise à jour et déplace l'incident à `status`. `201`. |
| `GET` | `/api/public/status/{slug}` | public | Le document JSON dont une page publiée est construite (aussi `badge.svg` et `rss` sous le même chemin). Dans `page`, `show_uptime_days` est le maximum configuré et `history_days` la fenêtre réellement montrée (l'ancienneté des données de la page, de 7 jours jusqu'à ce maximum) et `simple` reflète le mode simple. Voir [Pages d'état](../using/status-pages.md). Sur le domaine personnalisé d'une page (`Host` égal à son `domain`), seules les routes `/api/public/status/{slug}/…` de cette page répondent ; chaque autre route de cette référence est `404` là. |
| `GET` | `/api/public/status/{slug}/uptime.svg`, `/response.svg` | public | Badges de page : disponibilité moyenne sur `?days=` `1`, `7`, `30` (défaut) ou `90` — `400` au-delà de l'historique de la page — et temps de réponse moyen. SVG, `Cache-Control: public, max-age=60`. |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`, `/uptime.svg`, `/response.svg` | public | Les mêmes trois badges pour un service ; `key` est la `key` du service dans le document public (son libellé sous forme d'URL). `404` pour une clé inconnue. |
| `GET` | `/api/public/status/{slug}/logo` | public | Le logo de la page avec son type d'image, mis en cache un jour (le `logo_url` du document porte une version). `404` sans en avoir. |
| `POST` | `/api/public/status/{slug}/subscribe` | public | `{"email": "…"}`. Envoie un lien de confirmation via le canal SMTP de la page. Toujours `202` avec le même message, quel que soit l'état de l'adresse ; `400` pour une adresse invalide ; `404` quand la page n'offre pas d'abonnement email ; `409` quand la page ne peut plus prendre d'abonnés ; `429` avec `Retry-After` au-delà de cinq requêtes par client et soixante en tout par 15 minutes. |
| `POST` | `/api/public/status/{slug}/confirm?token=…` | public | Confirme un abonnement (le lien de l'email de confirmation ouvre `/s/{slug}/confirm`, dont le bouton poste ici). `404` pour un jeton inconnu ou expiré. |
| `POST` | `/api/public/status/{slug}/unsubscribe?token=…` | public | Retire un abonnement ; le corps est ignoré, c'est donc aussi la cible one-click RFC 8058 de `List-Unsubscribe`. Toujours `200`. |

## Jetons d'agent et ingestion

| Méthode | Route | Auth | Objet |
|---|---|---|---|
| `GET` | `/api/agent/tokens` | session seulement | Chaque jeton : `id`, `name`, `prefix`, `created_at`, `last_used_at`, `revoked_at`, `max_uses` (`null` pour un jeton de parc), `uses`, `expires_at`. |
| `POST` | `/api/agent/tokens` | admin, session seulement | `{"name": "Home fleet", "base_url": "http://server:8080", "reusable": false, "max_uses": null, "expires_in_days": null}`. `201` avec les champs du jeton plus `secret` (montré une fois), `install_linux` et `install_windows`. `base_url` est l'URL que les agents utiliseront ; vaut par défaut l'adresse d'écoute. Sans `reusable`, le jeton est **à usage unique** : il enrôle une machine et pas plus. Avec, `max_uses` plafonne les enrôlements (`null` : pas de limite). `expires_in_days` arrête les *enrôlements* après ce nombre de jours ; les machines déjà enrôlées continuent de rapporter. `400` sur un compteur sous 1. |
| `DELETE` | `/api/agent/tokens/{id}` | admin, session seulement | Révoquer. `204` ; `404` quand inconnu ou déjà révoqué. |
| `POST` | `/api/ingest` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Reçoit un lot d'échantillons d'un agent (corps jusqu'à 16 Mo). Pas destiné à être appelé à la main. L'en-tête porte le secret de liaison de la machine ; un agent qui n'en a pas l'omet, ce qui est comment il en demande un. La réponse ajoute `agent_secret` (le secret, renvoyé exactement une fois, quand la machine est liée) et `bound`. `403` quand la machine est liée à une autre installation d'agent, ou quand le jeton ne peut plus enrôler — le message dit lequel, et quoi faire. |
| `GET` | `/api/agent/relay?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Sondes déléguées à un agent relais (`relay: true`). Retenue jusqu'à `wait` secondes (25 au plus) quand rien n'est en attente. Chaque élément est une commande de type `probe` dont `args` porte la cible, son identifiant déchiffré, `timeout_secs` et `discover`. Jamais écrit sur disque. |
| `POST` | `/api/agent/relay/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Résultat d'une sonde déléguée : `{"duration_ms", "error", "samples", "profile_id"}` (corps jusqu'à 16 Mo). `204` ; `404` quand la sonde a expiré ou appartient à un autre agent. |
| `GET` | `/api/agent/commands?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Commandes en file pour un agent (redémarrage ou mise à jour de conteneur), retenue jusqu'à `wait` secondes quand rien n'est en attente. Pas destiné à être appelé à la main. `key` seule ne prouve rien — elle est publique — une machine liée doit donc aussi présenter son secret de liaison ; `403` sinon, ce qui empêche une machine de prendre les commandes d'une autre. |
| `POST` | `/api/agent/commands/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Progrès et résultat d'une commande, rapporté par l'agent. `result` est tronqué aux derniers 4 Ko, avec `[truncated by the server, beginning dropped]` au début quand ça arrive. |
| `GET` | `/api/relays` | session | Chaque appareil agent comme relais possible : `id`, `name`, `site`, `relay` (déclaré `relay: true`), `last_seen_at`, `relayed` (appareils atteints via lui). Les relais d'abord, puis par nom. |

```json
{
  "id": 1, "name": "Home fleet", "prefix": "dmon_ab12",
  "created_at": "2026-09-15 13:00:00", "last_used_at": null, "revoked_at": null,
  "secret": "dmon_…",
  "install_linux": "curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_… --url=http://server:8080",
  "install_windows": "& ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_… -Url http://server:8080"
}
```

## Conteneurs et commandes (appareils agent)

Actions sur les conteneurs d'une machine qui fait tourner l'agent. Chaque
route répond `404` quand l'appareil n'est pas une target `agent`.

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/targets/{id}/agent` | La machine telle que son agent l'a décrite pour la dernière fois : `hostname`, `os`, `os_version`, `arch`, `agent_version`, `commands_supported`, `relay`, `site`, `relayed` (appareils atteints via cet agent), `last_seen_at`, `binding`, `bound`, `bound_at`, `rebind_until`. `404` jusqu'à ce qu'un agent ait rapporté. `commands_supported` est `true` seulement quand l'agent a déclaré qu'il récupère des commandes (`commands: true`, le défaut des agents actuels) ; un agent plus ancien ou un avec `commands: false` donne `false`. `binding` est `bound`, `pending` (le binaire peut être lié et le sera à son prochain lot) ou `unsupported` (un agent plus ancien que la liaison : réinstallez-le). |
| `POST` | `/api/targets/{id}/agent/rebind` | Admin. Ouvre une fenêtre d'une heure pendant laquelle cette machine peut se lier à nouveau avec un jeton d'enrôlement valide — le chemin de retour après qu'une réinstallation a emporté le secret de l'agent. `{"rebind_until", "minutes"}`. La fenêtre se ferme dès qu'elle est utilisée, et l'ancien secret arrête de fonctionner. `404` jusqu'à ce qu'un agent ait rapporté. |
| `GET` | `/api/targets/{id}/containers` | Chaque conteneur du dernier lot : `name`, `image`, `up`, `health`, `restart_count`, `uptime_seconds`, `image_age_seconds`, `update_available`, `policy`, `last_command`. |
| `PUT` | `/api/targets/{id}/containers/{name}/policy` | `{"auto_restart": bool, "auto_update": bool, "prune_old_image": bool, "only_in_maintenance": bool}`. Chaque champ est optionnel : un champ omis garde sa valeur enregistrée. Renvoie la politique complète. |
| `POST` | `/api/targets/{id}/containers/{name}/restart` | Met en file un redémarrage. `201` avec la commande. `404` quand `name` n'est pas dans l'inventaire de l'agent ; `409` quand la même commande est déjà en file ou en cours, ou quand l'agent ne peut pas lancer de commandes (voir `commands_supported`). |
| `POST` | `/api/targets/{id}/containers/{name}/update` | Met en file une mise à jour ; `{"prune": bool}` optionnel (vaut par défaut la politique). Mêmes codes de statut que restart. |
| `GET` | `/api/targets/{id}/commands` | Les 20 dernières commandes, les plus récentes d'abord : `id`, `kind` (`container.restart`, `container.update`), `args`, `status`, `requested_by` (un utilisateur, ou `policy`), `created_at`, `started_at`, `finished_at`, `result`. |
| `DELETE` | `/api/targets/{id}/commands/{command_id}` | Annule une commande en file. `204` ; `409` une fois que l'agent l'a récupérée ou quand elle est déjà terminée ; `404` quand elle n'appartient pas à cet appareil. |

`status` passe par `queued → running → done | failed`. Deux autres états
finaux existent : `cancelled` (une personne a retiré la commande) et
`expired` (le serveur a abandonné après dix minutes parce qu'aucun agent
n'est venu la chercher — agent arrêté, trop ancien pour le canal de
commande, ou installé avec `commands: false`). Le serveur fait expirer les
commandes périmées chaque minute de lui-même ; une commande en file ne
bloque jamais une nouvelle plus longtemps que ça.

## Invités Proxmox VE

Les lignes du panneau **Invités** d'un appareil Proxmox VE, assemblées
depuis la dernière sonde dans le magasin de métriques (quatre requêtes
instantanées, rien demandé à l'hyperviseur). `404` quand l'appareil
n'existe pas, `400` quand ce n'est pas une target `proxmox`.

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/guests` | Un objet par VM ou conteneur, trié par nœud puis VMID : `vmid`, `name`, `node`, `kind` (`qemu`, `lxc`), `status` (`running`, `stopped`, `paused`, `suspended`, `template`, `unknown`), `cpu_percent` (des cœurs alloués), `cpu_count`, `memory_used_bytes`, `memory_total_bytes`, `memory_percent`, `balloon_bytes`, `disk_used_bytes`, `disk_total_bytes`, `disk_percent`, `agent` (`true` l'agent invité a répondu, `false` activé mais silencieux, `null` aucun), `network_in_bps`, `network_out_bps`, `disk_read_bps`, `disk_write_bps`, `uptime_seconds`, `last_backup_age_seconds`, `ha_state`, `pool` (son pool de ressources), `lock` (seulement tant qu'un verrou est tenu : `backup`, `migrate`…), `os` et `ip` (vus depuis l'intérieur de l'invité, rafraîchis chaque heure). Chaque valeur inconnue est `null` — une VM sans agent invité a `disk_total_bytes` mais `disk_used_bytes: null` ; un invité arrêté garde ses tailles et perd ses mesures. |

```json
[
  {
    "vmid": 202, "name": "nextcloud", "node": "pve2", "kind": "lxc", "status": "running",
    "cpu_percent": 1.0, "cpu_count": 4,
    "memory_used_bytes": 1879048192, "memory_total_bytes": 4294967296, "memory_percent": 43.75, "balloon_bytes": null,
    "disk_used_bytes": 61203283968, "disk_total_bytes": 107374182400, "disk_percent": 57.0, "agent": null,
    "network_in_bps": 1024.0, "network_out_bps": 512.0, "disk_read_bps": 0.0, "disk_write_bps": 2048.0,
    "uptime_seconds": 3196800, "last_backup_age_seconds": 25200, "ha_state": "started",
    "pool": "production", "lock": null,
    "os": "Debian GNU/Linux 12 (bookworm)", "ip": "192.168.10.60"
  }
]
```

## Nœuds et Ceph de Proxmox VE

Les sections **Nœuds** et **Ceph** de la même page d'appareil, lues depuis
la même dernière sonde. Mêmes codes de statut que ci-dessus.

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/nodes` | `{"nodes": […], "fencing_state", "fencing_armed"}` — l'état du watchdog HA (`armed`, `standby`…) et s'il isolerait vraiment un nœud perdu, puis un objet par nœud, trié par nom : `name`, `up`, `cpu_percent`, `cpu_iowait_percent`, `memory_percent`, `ksm_shared_bytes`, `rootfs_percent`, `uptime_seconds`, `version` (la release `pve-manager` installée sur ce nœud), `services_down` (noms d'unité des démons arrêtés ou en échec), `interfaces_offline` (interfaces réglées pour démarrer au boot qui ne sont pas actives), `thin_pools` (`name`, `vg`, `used_percent`, `metadata_used_percent`, `size_bytes`) et `volume_groups` (`name`, `used_percent`, `size_bytes`). Un nœud qui a arrêté de répondre garde son entrée avec `up: false`. |
| `GET` | `/api/targets/{id}/proxmox/ceph` | `available` (`false` quand le cluster n'a pas Ceph — l'UI ne dessine alors aucune section du tout), `health` (0 OK, 1 WARN, 2 ERR, 3 inconnu), `health_status`, `bytes_used`, `bytes_total`, `used_percent`, `osds_total`, `osds_up`, `osds_in`, `osds` (`name`, `host`, `device_class`, `up`, `in`, `used_percent`, `used_bytes`, `total_bytes`, `apply_latency_ms`, `commit_latency_ms`), `pools` (`name`, `used_percent`, `used_bytes`, `size`, `min_size`, `pg_num`, `pg_num_optimal`, `autoscale`), `filesystems` (noms CephFS), `flags` (drapeaux OSD actuellement réglés, comme `noout`) et `muted_checks` (contrôles de santé mis en sourdine, que `HEALTH_OK` ne mentionne plus). |

## Proxmox Backup Server

Les panneaux d'un appareil `pbs`, lus depuis la dernière sonde (rien
demandé au serveur sauf le journal de tâche). `404` quand l'appareil
n'existe pas, `400` quand ce n'est pas une target `pbs`.

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/targets/{id}/pbs/calendar?days=30&offset=120` | `{"probed_at", "days", "offset_minutes", "groups": […]}` : une entrée par groupe de sauvegarde (`datastore`, `namespace`, `backup_type`, `backup_id`, `name`, `count`, `last_time`, `last_size`, `last_verified`, `last_success`, `last_failure`, `retention`) avec un tableau `days` de `{date, state, runs, snapshot}`. `offset` est le décalage UTC du visiteur en minutes, pour que les jours soient découpés à minuit local. |
| `GET` | `/api/targets/{id}/pbs/failures?days=30` | Tâches en échec : `upid`, `worker_type`, `kind`, `worker_id`, `datastore`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pbs/jobs` | `{"probed_at", "jobs": […]}` : jobs sync, verify, prune et ramasse-miettes avec leur calendrier et dernier résultat. |
| `GET` | `/api/targets/{id}/pbs/health` | Datastores (usage, date de remplissage estimée), disques (SMART, usure) et pools ZFS. |
| `GET` | `/api/targets/{id}/pbs/tasks/{upid}/log?lines=200` | Le journal d'une tâche, récupéré depuis le serveur : `{"upid", "lines": […]}` ; `lines` plafonne combien sont renvoyées. |
| `GET` | `/api/targets/{id}/pbs/disks/smart?disk=/dev/sda` | Attributs SMART d'un disque ; `disk` est requis (`400` sans lui). |
| `GET` | `/api/targets/{id}/pdm/remotes` | Proxmox Datacenter Manager : totaux de parc (`estate`) et une ligne par instance fédérée (`id`, `kind`, `reachable`, `error`, `version`, `version_behind`, compteurs de nœud et d'invité, mémoire et stockage, `subscription`, `last_collection`, `tasks_failed`), les inaccessibles d'abord. |
| `GET` | `/api/targets/{id}/pdm/failures?days=14` | Tâches qui ont échoué à travers le parc : `upid`, `remote`, `worker_type`, `kind`, `worker_id`, `node`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pdm/health` | L'hôte de la console : CPU, mémoire, système de fichiers racine, uptime, certificats, mises à jour en attente et abonnement. |
| `GET` | `/api/targets/{id}/pmg/queues` | Proxmox Mail Gateway : les quatre files Postfix dans l'ordre de lecture (`queue`, `messages`, `domains`, `oldest_age_seconds`, `top_domains`, `stuck`), plus `total_messages` et un `stuck` à l'échelle de la passerelle. `oldest_age_seconds` est une borne inférieure : qshape rapporte des tranches d'âge. |
| `GET` | `/api/targets/{id}/pmg/traffic` | Les totaux du jour (`mail`), la courbe de trafic récente (`recent`), l'histogramme de score de spam, les virus attrapés aujourd'hui, et les compteurs de quarantaine (`quarantine`). Compteurs seulement : aucun contenu de message. |
| `GET` | `/api/targets/{id}/pmg/health` | Une entrée par nœud (système, services, `signatures` avec `family`, `age_seconds` et `stale`, `expiring_certificates`, mises à jour, abonnement), les membres `cluster` avec leur état de synchro, et `stopped_services` à travers les nœuds. |

## Synology

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/api/targets/{id}/synology` | Vue d'ensemble d'un appareil `synology` depuis la dernière sonde : `system` (modèle, version DSM, uptime, température, CPU, mémoire), `volumes` (`name`, `fs_type`, `raid_type`, `status`, `severity`, `total_bytes`, `used_bytes`), `disks` (`model`, `serial`, `kind`, `ssd`, `status`, `smart_status`, `temperature_celsius`, `size_bytes`, `remaining_life_percent`, …) et `sampled_at`. `404` quand l'appareil n'existe pas, `400` quand ce n'est pas un Synology. |
| `GET` | `/api/targets/{id}/synology/abb` | Active Backup for Business : `tasks`, et `devices` — un rapport par appareil protégé avec sa cadence apprise dans `assessment` (`state`, `last_success_s`, `typical_interval_s`, compteurs sur 30 jours). |

## Battements de cœur (moniteurs push)

| Méthode | Route | Auth | Objet |
|---|---|---|---|
| `GET` ou `POST` | `/api/push/{token}` | aucune (le jeton) | L'appel qu'une tâche fait chaque fois qu'elle tourne. `?status=up\|down&msg=…` optionnel (compatible Uptime Kuma). `204` vide sur succès, `404` pour un jeton inconnu, `400` pour un autre `status`, `429` avec `Retry-After` au-delà de soixante appels par minute par jeton. Voir [Battement de cœur](../devices/push.md). |
| `GET` | `/api/targets/{id}/push` | session | Le moniteur d'un appareil `push` : `token`, `path` (`/api/push/<token>`), `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `settings_error`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`). Créé à la première lecture. `400` quand l'appareil n'est pas un battement de cœur. |
| `POST` | `/api/targets/{id}/push/regenerate` | admin | Nouveau jeton ; l'URL précédente répond `404` à partir de là. Même corps que la lecture. |

## Assistants (MCP)

Le serveur [Model Context Protocol](https://modelcontextprotocol.io)
intégré, celui à qui un assistant parle. Il s'authentifie avec les jetons
d'API ci-dessus — mêmes portées, expiration, réseaux et limite de débit —
et jamais avec une session.

| Méthode | Route | Auth | Objet |
|---|---|---|---|
| `POST` | `/api/mcp` | jeton d'API | JSON-RPC 2.0 sur le transport Streamable HTTP, une requête par `POST`, répondue en JSON. Protocole `2026-07-28` (sans état : chaque requête porte sa version et les capacités du client dans `_meta`, plus les en-têtes `MCP-Protocol-Version`, `Mcp-Method` et `Mcp-Name` ; `server/discover`), et `2025-11-25`, `2025-06-18` et `2025-03-26` via la poignée de main `initialize`. Pas de session : aucun `Mcp-Session-Id` n'est émis, un envoyé est ignoré. Une notification obtient `202` et pas de corps. |
| `GET`, `DELETE` | `/api/mcp` | public | `405` avec `Allow: POST` — il n'y a pas de flux envoyé par le serveur ni de session à terminer ; le corps dit seulement ce qu'est ce point de terminaison. |

`tools` est la seule capacité — pas de resources, pas de prompts, pas
d'OAuth. Des 27 outils, un jeton `read` peut appeler les 15 qui ne font
que lire (`get_status`, `list_devices`, `get_device`, `list_alerts`,
`alert_history`, `query_metrics`, `list_silences`, `list_rules`,
`list_device_types`, `list_agents`, `list_containers`,
`list_heartbeats`, `list_status_pages`, `list_channels`, `list_packs`) ;
les 12 qui changent quelque chose (`silence_device`, `remove_silence`,
`acknowledge_alert`, `probe_device`, `set_device_enabled`,
`set_rule_enabled`, `add_device`, `discover_network`,
`schedule_maintenance`, `restart_container`, `test_channel`,
`post_incident`) ont besoin d'un jeton `write`. Les outils appellent le
même code que l'UI web. Versions, en-têtes, codes d'erreur et notes de
sécurité sont dans
[Connecter un assistant](../using/assistant.md#protocol-details).

## Fichiers d'agent (hors `/api`)

| Méthode | Route | Objet |
|---|---|---|
| `GET` | `/install.sh` | L'installateur Linux, macOS et FreeBSD. Public. |
| `GET` | `/install.ps1` | L'installateur Windows. Public. |
| `GET` | `/download/{name}` | `dumbmonit-agent-linux-x86_64`, `dumbmonit-agent-linux-aarch64`, `dumbmonit-agent-freebsd-x86_64`, `dumbmonit-agent-windows-x86_64.exe`, servis depuis `DUMBMONIT_AGENT_DIR` ; `{name}.sha256` donne sa somme de contrôle au format `sha256sum`. `404` si le fichier est absent. `dumbmonit-agent-macos-aarch64` et `dumbmonit-agent-macos-x86_64` (et leurs `.sha256`) répondent `307` vers le même fichier sur la dernière release GitHub. Public. |

Chaque autre chemin est servi par l'UI web, qui vous demande de vous
connecter elle-même.

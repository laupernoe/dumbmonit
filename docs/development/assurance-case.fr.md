# Argumentaire d'assurance

Cette page défend l'idée que DumbMonit respecte ses exigences de sécurité : elle
énonce les menaces, les frontières où la confiance change, pourquoi la
conception limite les dégâts d'une erreur, comment chaque grande classe de
faiblesse est contrée et quelles preuves soutiennent chaque affirmation. Elle
complète le [modèle de sécurité](security-model.md), qui énumère les mêmes
mesures plus brièvement. Pour signaler une vulnérabilité, suivez la
[politique de sécurité](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

Les chemins ci-dessous sont relatifs à la racine du dépôt.

## Exigences de sécurité

1. Seul un utilisateur authentifié, dont le rôle l'autorise, peut lire ou
   modifier la configuration, les alertes ou les mesures.
2. Les identifiants que DumbMonit conserve pour vos surveillances ne sont lisibles
   ni dans le fichier de base, ni dans une sauvegarde sans sa phrase de passe,
   ni par l'API.
3. Seule une machine munie d'un jeton valide peut envoyer des mesures, et un
   jeton qui fuit se révoque seul.
4. DumbMonit ne peut pas servir à atteindre des services qu'il ne devrait pas
   joindre, en particulier son VictoriaMetrics enfant, sans authentification.
5. Une page servie à un navigateur ne peut pas exécuter de script qu'elle n'a
   pas elle-même livré, et un site tiers ne peut pas faire modifier quoi que ce
   soit à un navigateur connecté.

## Modèle de menace

| Attaquant | Capacité | Dans le périmètre |
|---|---|---|
| Client réseau anonyme | Atteint le port HTTP | Oui |
| Utilisateur connecté à faible rôle | Session valide ou jeton d'API en lecture | Oui (élévation de privilèges) |
| Page web malveillante | S'exécute dans le navigateur d'un admin | Oui (CSRF, XSS, clickjacking) |
| Cible surveillée compromise ou malveillante | Répond aux sondes avec du contenu hostile | Oui (analyseurs, redirections) |
| Agent malveillant ou volé | Détient un jeton d'agent | Oui (portée d'un seul jeton) |
| Admin saisissant une URL hostile | Configure un webhook ou une adresse de surveillance | Oui, pour les adresses du serveur lui-même |
| Qui contrôle l'hôte, le volume `/data` ou le moteur de conteneurs | Contrôle total | Non : hors périmètre, comme dans la politique de sécurité |
| Failles de VictoriaMetrics ou des surveillances | Code tiers | Non : à signaler en amont |

## Frontières de confiance

| Frontière | Ce qui la traverse | Comment elle est gardée | Preuves |
|---|---|---|---|
| Navigateur vers `/api/*` | Cookie de session ou jeton d'API, JSON | Un seul intergiciel d'authentification, trois rôles, en-tête CSRF sur les écritures par cookie | `crates/server/src/auth/middleware.rs` ; tests `auth_api.rs`, `operator_role_api.rs`, `api_open_security_api.rs`, `cors_policy_api.rs` |
| Navigateur vers les pages servies | HTML, script, style | Content Security Policy avec nonce par réponse, `nosniff`, `Referrer-Policy: same-origin`, `X-Frame-Options: DENY` | `crates/server/src/api/mod.rs` (couche d'en-têtes de sécurité) |
| Fournisseur d'identité vers le serveur | Redirection OpenID Connect et jeton d'identité | State, nonce et PKCE (S256) ; le rattachement à un compte local est explicite | `crates/server/src/auth/oidc/` ; test `oidc_api.rs` |
| Agent vers `/api/ingest` | Mesures, résultats de commandes | Jeton par agent, conservé sous forme d'empreinte SHA-256 (préfixe `dmon_`) | `crates/server/src/api/ingest.rs`, `crates/server/src/collectors/agent/token.rs` ; test `agent_binding_api.rs` |
| Agent relais vers le serveur | Résultats de sondes d'un autre réseau | Même vérification de jeton d'agent (`agent_commands::authenticate`) | `crates/server/src/api/relay.rs` ; test `relay_api.rs` |
| Serveur vers les surveillances | Requêtes de sonde, réponses hostiles | Redirections suivies sur le même hôte seulement, nombre borné, analyseurs testés, vérification des certificats par défaut | `crates/collectors/src/http.rs` (`same_host_redirects`) |
| Serveur vers les canaux de notification | Requêtes webhook et SMTP | Même politique de redirection, plus une garde qui refuse les adresses du serveur lui-même, vérifiée avant l'envoi, à chaque redirection et à la résolution DNS | `crates/server/src/notify/guard.rs` ; test `notify_guard_api.rs` |
| Serveur vers VictoriaMetrics | Écritures et requêtes de lecture | L'enfant embarqué écoute sur `127.0.0.1:8428` par défaut ; le relais compatible Prometheus ne transmet que les routes de lecture (query, query_range, series, labels) | `crates/server/src/config.rs` (`DUMBMONIT_VM_LISTEN`), `crates/server/src/api/prometheus.rs` (`readable_segments`) |
| Serveur vers SQLite | Configuration et état | Paramètres liés via `sqlx` | `crates/server/src/db/` |
| Serveur vers `/data` | Base, clé secrète | Secrets chiffrés en AES-256-GCM ; l'utilisateur du conteneur ne possède que `/data` | `crates/server/src/crypto.rs`, `Dockerfile` |

## Argument de conception sûre

L'argument suit les principes de conception de Saltzer et Schroeder.

- **Économie de mécanisme.** Un binaire, un fichier SQLite, un processus enfant
  en boucle locale, une seule entrée pour tous les clients (`/api/*`).
- **Valeurs par défaut sûres.** Sur une instance neuve, toute route derrière la
  garde d'authentification répond `401` ; seuls la santé, l'état
  d'authentification, la création du premier compte et la connexion répondent,
  donc personne ne peut préparer l'instance avant son propriétaire
  (`auth/middleware.rs`, test `api_open_security_api.rs`). VictoriaMetrics écoute
  en boucle locale sauf si l'opérateur en décide autrement. Les certificats TLS
  des services surveillés sont vérifiés sauf si l'opérateur active
  `insecure_tls` (`crates/collectors/src/http.rs`).
- **Médiation complète.** L'intergiciel d'authentification couvre toutes les
  routes de l'API ; les gestionnaires n'ont pas à vérifier chacun de leur côté.
- **Conception ouverte.** Le code, la description de l'API
  (`crates/server/src/api/openapi.yaml`) et cette page sont publics. La sécurité
  ne repose pas sur le secret de la conception.
- **Séparation des privilèges.** Sessions, jetons d'API et jetons d'agent sont
  des identifiants différents. Les jetons d'API ont une portée `read` ou
  `write` (`auth/token.rs`), et les utilisateurs un rôle lecteur, opérateur ou
  admin.
- **Moindre privilège.** L'image s'exécute sous l'UID 65532 (`Dockerfile`). Démarré sous root (`user:
  "0:0"`, pour les bind mounts), le serveur se contente de transférer le
  répertoire de données à cet utilisateur, sans suivre les liens, puis abandonne
  root définitivement et vérifie qu'il ne peut pas le retrouver, avant d'ouvrir
  quoi que ce soit d'autre (`privileges.rs`). Le fichier Compose ne garde que
  les quatre capacités dont cette étape a besoin, monte le système de fichiers
  racine en lecture seule et active `no-new-privileges` (`docker-compose.yml`).
- **Moindre mécanisme commun.** Les agents s'authentifient avec leur propre
  jeton : révoquer une machine ne touche ni utilisateur ni autre agent.
- **Acceptabilité psychologique.** Le comportement sûr est le défaut et ne
  demande rien de plus : le cookie est `HttpOnly` et `SameSite=Lax`, l'en-tête
  CSRF est ajouté par le client web, les secrets sont en écriture seule dans
  l'interface.
- **Défense en profondeur.** Chiffrement au repos en plus du contrôle d'accès ;
  en-tête CSRF en plus de `SameSite` ; limitation des connexions en plus du
  hachage Argon2 ; politique de redirection en plus de la garde des adresses
  propres.

### Cryptographie

- Mots de passe : Argon2 (`auth/password.rs`). Les secrets de session et de
  jeton d'API sont conservés sous forme d'empreintes SHA-256 de valeurs
  aléatoires (`auth/session.rs`, `auth/token.rs`).
- Secrets au repos : AES-256-GCM avec un nonce neuf à chaque chiffrement
  (`crypto.rs`). La clé est dérivée par Argon2id de `DUMBMONIT_SECRET` ou du
  fichier de clé généré, avec un sel persistant ; un secret de moins de 16
  caractères est refusé, et une valeur témoin détecte une mauvaise clé au
  démarrage.
- Sauvegardes : la clé dérive de la phrase de passe de l'opérateur par Argon2id,
  paramètres inscrits dans le lot, ce qui permet de changer d'algorithme sans
  casser les anciens lots (`backup/bundle.rs`).
- Second facteur : TOTP, avec des codes de secours à usage unique conservés sous
  forme d'empreintes (`auth/totp.rs`).
- OpenID Connect : state, nonce et PKCE `S256` (`auth/oidc/pkce.rs`).
- Toutes les primitives viennent de bibliothèques maintenues (`aes-gcm`,
  `argon2`, `sha2`) ; aucune n'est faite maison. Le serveur intégré parle en
  HTTP clair ; la documentation impose un reverse proxy TLS, et les cookies
  reçoivent `Secure` quand la requête arrive en HTTPS (`auth/cookie.rs`).

## Parades aux faiblesses courantes

| Faiblesse (OWASP Top 10, CWE) | Parade | Preuves |
|---|---|---|
| Contrôle d'accès défaillant, CWE-862, CWE-285 | Intergiciel d'authentification, rôles, portées de jetons | `operator_role_api.rs`, `api_tokens_api.rs`, `api_open_security_api.rs` |
| Défaillances cryptographiques, CWE-327, CWE-312 | AES-256-GCM au repos, Argon2, aucun secret renvoyé par l'API | tests unitaires de `crypto.rs` ; `has_secret` / `credential_kind` au lieu du secret |
| Injection SQL, CWE-89 | Paramètres liés via `sqlx` | `crates/server/src/db/` |
| Cross-site scripting, CWE-79 | Svelte échappe les sorties ; CSP avec `default-src 'none'`, scripts depuis `'self'` et un nonce uniquement ; `base-uri 'none'` | `api/mod.rs` |
| Cross-site request forgery, CWE-352 | Les écritures par cookie exigent `X-Requested-With: DumbMonit` ou les métadonnées de requête de même origine ; le CORS n'accorde l'en-tête à personne ; `SameSite=Lax` | `auth/middleware.rs`, `auth/cors.rs` ; test `cors_policy_api.rs` |
| Détournement de clic, CWE-1021 | `X-Frame-Options: DENY` et `frame-ancestors 'none'`, sauf sur les pages de statut publiques activées à dessein pour être intégrées | `api/mod.rs` |
| Falsification de requête côté serveur, CWE-918 | Redirections sur le même hôte seulement ; la garde des notifications refuse les adresses d'écoute du serveur et de VictoriaMetrics, y compris après résolution DNS (rebinding). La boucle locale et les plages privées restent joignables volontairement, car les services d'un homelab y vivent | `notify/guard.rs` ; test `notify_guard_api.rs` |
| Défauts d'authentification, CWE-307, CWE-287 | Limitation des connexions à délai croissant par adresse et par compte, TOTP optionnel, OIDC avec PKCE, réinitialisation du second facteur par un admin | `auth/rate_limit.rs`, `auth/totp.rs`, `auth/oidc/` |
| Désérialisation non sûre, épuisement de ressources, CWE-400 | JSON typé via `serde`, corps de requête plafonnés à 16 Mio, redirections bornées | `api/mod.rs`, `collectors/src/http.rs` |
| Sûreté mémoire, CWE-787, CWE-416 | Rust ; `unsafe` n'apparaît que dans le code des compteurs de performance de l'agent Windows (`crates/agent/src/collect/perf_counters.rs`) | recherche de `unsafe` |
| Composants vulnérables, CWE-1104 | `cargo deny` (licences, interdits, avis) ; Dependabot ; analyse Trivy des images publiées ; actions épinglées par commit ; builds `--locked` | `.github/workflows/ci.yml`, `security.yml`, `.github/dependabot.yml`, `deny.toml` |
| Intégrité des logiciels et des données, CWE-494 | Les installeurs d'agent vérifient le SHA-256 du binaire téléchargé | `crates/agent/install/install.sh`, `install.ps1` |
| Journalisation insuffisante, CWE-778 | Journal de sécurité des connexions et des changements sensibles | `auth/audit.rs` ; test `security_audit_api.rs` |

Les risques résiduels connus figurent dans le modèle de sécurité (serveur en
HTTP clair, options de surveillance non chiffrées, VictoriaMetrics embarqué sans
authentification). La CSP autorise les styles en ligne (`style-src
'unsafe-inline'`), ce qui l'affaiblit contre l'injection de style mais pas
contre l'exécution de script. Un avis est accepté, `RUSTSEC-2023-0071`, ignoré
dans `deny.toml` avec sa raison : la crate `rsa` ne fait que vérifier des
signatures RS256 avec la clé publique du fournisseur et n'utilise jamais de clé
privée.

## Preuves

- **Intégration continue** (`.github/workflows/ci.yml`) : `cargo fmt --check`,
  `clippy -D warnings`, toute la suite de tests Rust, `cargo deny check licenses
  bans` et `cargo deny check advisories`, vérification de types, tests et build
  de l'interface web, et build de l'image Docker, à chaque push et pull request.
- **Analyses planifiées** (`.github/workflows/security.yml`) : Trivy sur les
  images serveur et agent publiées, et OpenSSF Scorecard, chaque semaine et après
  chaque publication de `:edge`. Les résultats apparaissent dans l'onglet
  Security du dépôt, où l'analyse de code GitHub (CodeQL) rapporte aussi.
- **Tests** : tests d'API, d'authentification, de rôles, de CORS, d'OIDC, de
  garde et d'audit sous `crates/server/tests/`, tests unitaires à côté du code,
  et tests web sous `web/tests/`. La couverture de lignes Rust mesurée avec
  `cargo-llvm-cov` est de 87,9 %.
- **Politique de régression** : un correctif s'accompagne d'un test qui échoue
  sans lui (voir la politique de tests dans `CONTRIBUTING.md`).
- **Trace publique** : les correctifs de sécurité sont publiés sous forme d'avis
  et listés dans les notes de version.

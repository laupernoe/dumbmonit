# Modèle de sécurité

Cette page décrit ce que DumbMonit protège, à qui il fait confiance, comment sa
conception limite les dégâts d'une erreur, et où regarder dans le code et les
tests. Pour signaler une vulnérabilité, suivez la
[politique de sécurité](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## Ce qui est en jeu

DumbMonit tourne dans votre réseau. Il conserve les identifiants nécessaires
pour lire chaque équipement (communautés SNMP, jetons d'API, mots de passe), il
reçoit les mesures des agents installés sur vos machines et il envoie des
requêtes vers les adresses que vous configurez. Une compromission exposerait
ces secrets et donnerait un point d'appui dans le réseau supervisé.

## Frontières de confiance

| Frontière | De l'autre côté | Confiance |
|---|---|---|
| Navigateur ou script vers `/api/*` | Admins, opérateurs et lecteurs | Authentifiés par cookie de session ou jeton d'API ; trois rôles limitent ce que chacun peut faire. |
| Agent vers `/api/ingest` | Machines où vous avez installé un agent | Un jeton par machine ; un jeton qui fuite se révoque seul. |
| Serveur vers équipements et services | Systèmes tiers | Non fiables : les réponses sont analysées avec prudence et les redirections restent sur le même hôte. |
| Serveur vers canaux de notification | Services externes | Identifiants chiffrés au repos, jamais renvoyés par l'API. |
| Serveur vers VictoriaMetrics | Processus enfant en boucle locale, ou instance externe | L'instance embarquée n'a pas d'authentification et écoute en boucle locale ; une instance externe relève de l'opérateur. |
| Volume `/data` et hôte | L'opérateur | De confiance. Un attaquant qui contrôle l'hôte ou le volume est hors périmètre. |

## Principes de conception sûre appliqués

- **Moindre privilège.** Le conteneur tourne sans privilège, sans capability,
  avec un système de fichiers racine en lecture seule et `no-new-privileges`.
  Trois rôles (lecteur, opérateur, admin) limitent les modifications
  (`crates/server/src/auth/users.rs` ; test `operator_role_api.rs`).
- **Sûr par défaut.** Sur une instance neuve, seules les routes de santé,
  d'installation, de connexion et d'état d'authentification répondent tant que
  le premier admin n'existe pas (test `api_open_security_api.rs`).
- **Économie de mécanisme.** Un binaire, un fichier SQLite, un processus enfant
  en boucle locale, une seule entrée pour tous les clients : `/api/*`.
- **Défauts sûrs.** Un contrôle de sécurité sans donnée est `unknown`, jamais
  une réussite ; un équipement injoignable et une erreur de configuration sont
  des états distincts.
- **Médiation complète.** Un seul intergiciel d'authentification
  (`crates/server/src/auth/middleware.rs`) protège les routes de l'API.
- **Séparation des privilèges.** Sessions, jetons d'API et jetons d'agent sont
  des identifiants différents, à la portée différente.
- **Défense en profondeur.** Chiffrement au repos en plus du contrôle d'accès,
  en-tête anti-CSRF en plus des attributs du cookie, limitation des essais en
  plus du hachage des mots de passe.
- **Aucun secret ne revient.** L'API renvoie `has_secret` et `credential_kind`,
  jamais le secret ; omettre un secret dans un `PUT` le conserve.
- **Cryptographie éprouvée uniquement.** Mots de passe hachés avec Argon2,
  secrets chiffrés en AES-256-GCM (`crypto.rs`), double authentification TOTP
  avec codes de secours hachés à usage unique, OpenID Connect avec state, nonce
  et PKCE. Rien n'est maison.

## Parades par classe de faiblesse

| Classe (thème OWASP Top 10) | Mesures |
|---|---|
| Contrôle d'accès défaillant | Un seul intergiciel d'authentification ; rôles ; une instance neuve refuse tout sauf installation, connexion, santé et état d'authentification. |
| Défaillances cryptographiques | AES-256-GCM au repos, clé hors de la base ; mots de passe hachés avec Argon2 ; le serveur intégré parle en HTTP clair, d'où un reverse proxy TLS documenté. |
| Injection | SQL via `sqlx` avec paramètres liés ; le proxy VictoriaMetrics ne transmet que des requêtes en lecture (`tsdb/mod.rs`, `proxy_read`) ; analyseurs testés unitairement. |
| Conception non sécurisée | Le modèle de menaces ci-dessus ; aucun secret ne revient ; instance à usage unique. |
| Mauvaise configuration | Fichiers Compose durcis, VictoriaMetrics en boucle locale, notes de durcissement dans la politique de sécurité. |
| Composants vulnérables | `cargo deny` contrôle licences, interdits et avis RustSec ; scan Trivy hebdomadaire des images publiées ; Dependabot ; GitHub Actions épinglées par commit. |
| Défauts d'identification et d'authentification | Limitation des essais par adresse client et par compte (`auth/rate_limit.rs`), double authentification facultative, OpenID Connect, un admin peut réinitialiser le second facteur d'un utilisateur et le déconnecter. |
| Intégrité des logiciels et des données | Les installateurs d'agent vérifient le SHA-256 du binaire téléchargé ; les images publiées sont construites par la CI. |
| Journalisation et surveillance | Un journal de sécurité des connexions et changements sensibles (`auth/audit.rs`). |
| Falsification de requête côté serveur | Intégrations et canaux ne suivent les redirections que vers le même hôte : un équipement ne peut pas détourner une requête et sa clé d'API ailleurs. |
| Falsification de requête inter-sites | Une requête qui modifie l'état et s'appuie sur le cookie doit porter `X-Requested-With: DumbMonit` ou des métadonnées de même origine, sinon `403` ; les requêtes par jeton n'ont pas de cookie. |

## Vérification

Chaque push et pull request exécute `clippy -D warnings`, la suite de tests
Rust (dont les tests d'API, d'authentification et de rôles dans
`crates/server/tests/`), les tests web dans `web/tests/`, les contrôles de
dépendances et l'analyse CodeQL. L'OpenSSF Scorecard tourne périodiquement. Un
correctif de sécurité vient avec un test de non-régression et figure, avec son
identifiant CVE ou GHSA, dans les notes de version.

L'[argumentaire d'assurance](assurance-case.md) détaille le modèle de menace,
l'argument de conception sûre et les preuves de chaque affirmation.

## Limites connues

- Le serveur intégré parle en HTTP clair ; placez du TLS devant.
- Les options d'équipement sont stockées en clair. Mettez les secrets dans les
  champs d'identifiants.
- Le VictoriaMetrics embarqué n'a pas d'authentification.
- DumbMonit est avant la 1.0 ; les routes et les champs peuvent encore changer.

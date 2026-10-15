# Développement

DumbMonit est un binaire Rust unique (`crates/server`, qui intègre la construction
SvelteKit) plus VictoriaMetrics pour les séries temporelles, lancé par le serveur
depuis le binaire livré dans l'image, et une base de données SQLite intégrée pour
la configuration et l'état. Un conteneur. Le code Rust et les commentaires sont
en français ; l'interface Web est en anglais.

## Disposition du projet

\`\`\`
crates/proto     types partagés: Sample, Target, Credential, trait Collector (+ ProbeError)
crates/server    le binaire
  api/           routes axum (mod.rs est la table des routes) ; spa.rs sert web/build
  auth/          mot de passe instance unique, cookie session HttpOnly, limite de taux
  collectors/    collecteurs partagés (snmp avec profiles/*.yaml chargés au démarrage), proxmox, pbs, synology, agent, uptime
  scheduler.rs   exécute chaque cible activée sur son intervalle via le registre de collecteurs
  tsdb/          écrivain VictoriaMetrics (envidage par lots) + proxy de requête ; embedded.rs supervise l'enfant VictoriaMetrics
  db/            SQLite + migrations numérotées dans db/migrations/
  alerting/      règles, machine d'états (Phase/EffectivePhase), suppression par parent, silences, ligne de base saisonnière
  notify/        22 canaux de notification, décrits à l'interface Web par notify/catalog.rs
  crypto.rs      AES-256-GCM pour les identifiants/jetons, clé dérivée de /data/secret.key ou DUMBMONIT_SECRET
crates/agent     Agent Linux/macOS/FreeBSD/Windows ; install/ détient install.sh/.ps1 servi par le serveur
web/             SvelteKit (runes Svelte 5, Tailwind 4, uPlot), adaptateur-statique repli SPA, ssr=false
profiles/        Profils de collection SNMP, auto-appliqués par sysObjectID
docs/            cette documentation (MkDocs, publiée sur Read the Docs)
\`\`\`

## Construction sans chaîne d'outils Rust locale

La compilation se fait dans Docker ; rien liée à Rust n'a besoin d'être installé sur l'hôte.

\`\`\`bash
# Pile complète (construit l'image: environ 10 min à froid) — UI sur http://localhost:8080
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build

# Rust, dans l'étape constructeur (musl/alpine)
docker build -t dumbmonit-devenv --target builder .
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server --test alerts_api   # un fichier de test d'intégration
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server test_name          # un test par nom
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo clippy --all-targets --all-features -- -D warnings
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo fmt --all --check
\`\`\`

Avec Rust installé localement, `cargo test` et `cargo clippy -- -D warnings`
fonctionnent directement. Édition 2024, MSRV 1.95 ; `rustfmt.toml` définit
`max_width = 100` et `use_small_heuristics = "Max"`.

La construction Docker est en couches : construction Web → dépendances Rust sur sources
factices → sources réelles (avec un `touch` avant la deuxième `cargo build`, sinon cargo
réutilise les artefacts factices) → agent compilé croisé avec cargo-zigbuild pour
`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` et
`x86_64-pc-windows-gnu` → `FROM scratch`. `web/build` doit exister avant
`cargo build`, ou le serveur se livre sans interface Web.

## Interface Web

Node 22 est suffisant sur l'hôte.

\`\`\`bash
cd web && npm run dev        # Vite sur :5173, proxies /api vers localhost:8080 (le serveur Docker)
cd web && npm run check      # svelte-check (CI exécute ceci)
cd web && npm run build      # construction statique dans web/build, intégrée par rust_embed au cargo build
\`\`\`

Conventions : runes Svelte 5 uniquement (`$props`, `$state`, `$derived`, `$effect`,
extraits, `onclick`). Tout accès réseau va par `src/lib/api/` ;
les composants ne `fetch` jamais. Les types dans `api/types.ts` reflètent exactement
les structures Rust et doivent rester synchronisés avec `crates/server/src/api/*.rs`.
Le système de conception vit dans `src/app.css` (jetons) et `src/lib/ui/` (primitives).
Ne lancez jamais deux `vite build` à la fois : ils effacent `.svelte-kit/output`.

## Traductions

L'interface Web se traduit par pull request, comme toute autre contribution. La langue source est l'anglais (`web/messages/en.json` : clés plates, paramètres `{param}`) ; chaque autre langue tient dans un fichier, `web/messages/<locale>.json`, compilé par [Paraglide JS](https://paraglidejs.com) en fonctions de messages (arborescence élaguée) sous `web/src/lib/paraglide/` (généré, non suivi).

1. Modifiez `web/messages/<locale>.json` (`fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans`), ou copiez `en.json` pour ajouter une nouvelle langue.
2. Conservez toutes les clés et tous les `{variables}` de `en.json` ; ne traduisez que les valeurs. Aucun balisage HTML dans un message.
3. Les pluriels utilisent la syntaxe de message, pas des chaînes assemblées à la main : gardez la structure `match`/`plural` du message anglais et traduisez chaque branche.
4. Lancez `cd web && npm run i18n:check` : il échoue si une langue n'a pas une clé ou change ses `{variables}`.
5. Ouvrez une pull request.

Les nouvelles chaînes d'interface sont d'abord ajoutées à `web/messages/en.json` (seul fichier requis pour une pull request de fonctionnalité ; `npm run i18n:merge` fusionne des fragments temporaires). Une langue apparaît dans le sélecteur (Paramètres, Apparence) dès que son fichier existe. Voir `CONTRIBUTING.md` pour la convention de nommage des clés.

`web/project.inlang/settings.json` pointe son un plugin (message-format) à
`./node_modules/@inlang/plugin-message-format/dist/index.js` plutôt que l'URL
jsdelivr que l'outillage propre d'inlang utilise par défaut : un chemin relatif,
non-`http` est lu directement du disque, donc `npm run build` ne joints jamais
à un CDN — important puisque la construction d'image n'a aucune garantie d'un cache
npm chaud offrant cette URL, et un outil de surveillance qui a besoin d'internet
pour se reconstruire est une mauvaise apparence.

## Tests et CI

`.github/workflows/ci.yml` exécute, sur chaque tirage et demande de tirage :

1. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features` ;
2. `cargo deny check licenses bans` — seules les licences approuvées OSI sont autorisées
   par `deny.toml` ; une dépendance copyleft ou source-disponible échoue la construction ;
3. `npm run check` et `npm run build` ;
4. l'image (`linux/amd64`), construction uniquement.

`.github/workflows/release.yml` pousse la même image vers
`ghcr.io/laupernoe/dumbmonit` : `:edge` à chaque tirage vers `main`, `:X.Y.Z`,
`:X.Y`, `:X` et `:latest` à chaque étiquette `v*`.

Les tests d'intégration vivent dans `crates/server/tests/` ; les modules purs
(conversion métrique, analyse des options, machine d'états d'alerte, silences)
sont testés en place sans réseau.

## Ajout d'un collecteur

1. Implémentez `Collector` de `crates/proto/src/collector.rs` : `kind()`,
   `probe(&Target) -> Result<Vec<Sample>, ProbeError>` et optionnellement
   `discover()` pour la détection de profil. Retournez les variantes
   `ProbeError` véridiquement : `means_down()` décide si une défaillance
   compte comme "surveillance arrêtée" (alerte) ou comme erreur de configuration
   (affichée sur la surveillance, non notifiée).
2. Lisez vos options de `Target::tags` dans un `options.rs`, avec des défauts.
3. Enregistrez-le dans `crates/server/src/main.rs` avec
   `registry.register(...)`. L'ordonnanceur et l'API ne connaissent jamais
   les types concrets.
4. Décrivez-le dans `crates/server/src/api/collectors.rs` : étiquette, résumé,
   exemples, types de compte, indication d'adresse, avis de configuration et la
   liste `options` (clé, étiquette, aide, espace réservé, défaut, entrée, choix).
   L'interface Web construit son formulaire à partir de ceci ; un test là vérifie
   que chaque type enregistré a un avis et que les clés d'option correspondent
   à ce que le collecteur lit.
5. Nommez les métriques `<what>_<unit>` sans le préfixe `dumbmonit_` (l'écrivain
   l'ajoute) et ajoutez toute métrique nouvelle qu'une règle intégrée cible à la
   liste dans les tests `alerting/rules.rs`.
6. Ajoutez une page sous `docs/devices/`.

Les échantillons sont `Sample::new(name, value, MetricKind::Gauge | Counter,
timestamp_ms)` avec `.with_label(k, v)` ; les étiquettes d'identité (`target`,
`host`, `tag_*`) sont ajoutées par le registre. Ne les émettez pas vous-même :
nommez une étiquette par-objet d'après l'objet (`proxy_host`, `esxi`,
`ceph_host`). Une étiquette en collision est conservée comme `exported_<name>`,
et les options listées pour le type dans `api/collectors.rs` ne sont pas copiées
comme `tag_*`.

## Ajout d'un canal de notification

1. Implémentez `Notifier` (`crates/server/src/notify/mod.rs`) : `kind()` et
   `async fn send(&Message) -> Result<(), NotifyError>`. Les canaux Web
   partagent le client HTTP et les aides dans `notify/http.rs` ; regardez
   `services.rs`, `chat.rs`, `push.rs`, `oncall.rs` et `smtp.rs` pour les
   semblables.
2. Ajoutez le type à `CHANNEL_KINDS` et à la correspondance `build` dans
   `notify/mod.rs`, qui transforme un `ChannelConfig` stocké (paramètres +
   secrets déchiffrés) en `Notifier`.
3. Décrivez-le dans `notify/catalog.rs` : étiquette, résumé, ancre `doc_url`,
   et les champs `settings` et `secrets` (clé, étiquette, obligatoire, entrée,
   aide, espace réservé, options, forme, défaut). L'interface Web rend le
   formulaire à partir de cette description, donc aucune libération d'interface
   Web n'est nécessaire.
4. Documentez-le dans `docs/notifications.md` sous *Canaux soutenus*, avec une
   rubrique dont le slug correspond à l'ancre `doc_url` : l'interface Web
   établit directement un lien vers cette ancre sur Read the Docs.

## Captures d'écran sans tête

`web/tools/README.md` décrit comment rendre chaque page
dans un Chromium sans tête s'exécutant dans Docker (`zenika/alpine-chrome:with-puppeteer`,
aucun navigateur local nécessaire) : connectez-vous avec `curl` pour obtenir le
cookie `dumbmonit_session`, puis exécutez `web/tools/screenshot.js` contre le
serveur d'aperçu ou directement contre `:8080`. Elle écrit
`web/shots/<page>-<theme>-<viewport>.png` pour les deux thèmes et les deux
fenêtres de visualisation, c'est ainsi que l'interface Web est vérifiée après
un changement et comment les captures d'écran de cette documentation ont été prises.

## Documentation

Ce site est construit avec MkDocs et Material for MkDocs à partir de `docs/`
(`mkdocs.yml` à la racine, `.readthedocs.yaml` pour Read the Docs). Pour
obtenir un aperçu :

\`\`\`bash
pip install -r docs/requirements.txt
mkdocs serve
mkdocs build --strict   # ce que Read the Docs exécute ; doit réussir sans avertissements
\`\`\`

Chaque langue est un projet Read the Docs distinct (`dumbmonit` pour l'anglais, puis
`dumbmonit-fr`, `dumbmonit-de`, ... liés comme traductions), tous construits depuis le même
`mkdocs.yml`. `hooks/rtd_language.py` lit `READTHEDOCS_LANGUAGE` et construit cette seule langue
à la racine du site : les fichiers `page.<lang>.md` remplacent les pages anglaises, une page non
traduite reste en anglais avec un bandeau, et les titres de navigation viennent de
`hooks/nav_translations.yml`. En local la variable est absente et le build est en anglais ;
pour prévisualiser une autre langue : `READTHEDOCS_LANGUAGE=fr mkdocs serve` (`de`, `es`, `it`,
`pt`, `pt-br`, `ru`, `zh-cn`).

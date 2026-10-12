# Développement

DumbMonit est un seul binaire Rust (`crates/server`, qui embarque la
compilation SvelteKit) plus VictoriaMetrics pour les séries temporelles,
démarré par le serveur depuis le binaire fourni dans l'image, et une base
de données SQLite embarquée pour la configuration et l'état. Un
conteneur. Le code et les commentaires Rust sont en français ; l'interface
web est en anglais.

## Disposition du projet

```
crates/proto     types partagés : Sample, Target, Credential, trait Collector (+ ProbeError)
crates/server    le binaire
  api/           routes axum (mod.rs est la table de routes) ; spa.rs sert web/build
  auth/          mot de passe d'instance unique, cookie de session HttpOnly, limitation de débit
  collectors/    snmp (profiles/*.yaml chargés au démarrage), proxmox, pbs, synology, agent, uptime
  scheduler.rs   fait tourner chaque target activée sur son intervalle via le registre de collecteurs
  tsdb/          écrivain VictoriaMetrics (flush groupé) + proxy de requêtes ; embedded.rs supervise le VictoriaMetrics enfant
  db/            SQLite + migrations numérotées dans db/migrations/
  alerting/      règles, machine à états (Phase/EffectivePhase), suppression par parent, silences, référence saisonnière
  notify/        22 canaux de notification, décrits à l'UI par notify/catalog.rs
  crypto.rs      AES-256-GCM pour identifiants/jetons, clé dérivée de /data/secret.key ou DUMBMONIT_SECRET
crates/agent     agent Linux/macOS/FreeBSD/Windows ; install/ contient install.sh/.ps1 servis par le serveur
web/             SvelteKit (runes Svelte 5, Tailwind 4, uPlot), repli SPA adapter-static, ssr=false
profiles/        profils de collecte SNMP, appliqués automatiquement par sysObjectID
docs/            cette documentation (MkDocs, publiée sur Read the Docs)
```

## Compiler sans toolchain Rust

La compilation se fait dans Docker ; rien de Rust n'a besoin d'être
installé sur l'hôte.

```bash
# Stack complète (compile l'image : environ 10 min à froid) — UI sur http://localhost:8080
docker compose up -d --build
# surcouche développeur : le VictoriaMetrics embarqué est publié sur :8428 pour des requêtes MetricsQL directes
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build

# Rust, dans l'étape builder (musl/alpine)
docker build -t dumbmonit-devenv --target builder .
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server --test alerts_api   # un fichier de test d'intégration
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server test_name          # un test par son nom
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo clippy --all-targets --all-features -- -D warnings
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo fmt --all --check
```

Avec Rust installé localement, `cargo test` et
`cargo clippy -- -D warnings` fonctionnent directement. Édition 2024,
MSRV 1.88 ; `rustfmt.toml` règle `max_width = 100` et
`use_small_heuristics = "Max"`.

La compilation Docker est en couches : compilation web → dépendances Rust
sur des sources factices → vraies sources (avec un `touch` avant le
second `cargo build`, sinon cargo réutilise les artefacts factices) →
agent cross-compilé avec cargo-zigbuild pour
`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` et
`x86_64-pc-windows-gnu` → `FROM scratch`. `web/build` doit exister avant
`cargo build`, sinon le serveur livre sans interface.

## Interface web

Node 22 suffit sur l'hôte.

```bash
cd web && npm run dev        # Vite sur :5173, relaie /api vers localhost:8080 (le serveur Docker)
cd web && npm run check      # svelte-check (la CI le lance)
cd web && npm run build      # compilation statique dans web/build, embarquée par rust_embed à cargo build
```

Conventions : runes Svelte 5 seulement (`$props`, `$state`, `$derived`,
`$effect`, snippets, `onclick`). Tout l'accès réseau passe par
`src/lib/api/` ; les composants n'appellent jamais `fetch`. Les types dans
`api/types.ts` reflètent exactement les structures Rust et doivent rester
synchronisés avec `crates/server/src/api/*.rs`. Le système de design
(tokens dans `src/app.css`, primitives dans `src/lib/ui/`) est documenté
dans `DESIGN.md` ; le contexte produit dans `PRODUCT.md`. Ne lancez jamais
deux `vite build` à la fois : ils vident `.svelte-kit/output`.

## Traductions (Weblate)

L'interface web est câblée pour la traduction communautaire mais rien
n'est encore traduit : l'anglais est la seule locale fournie, dans
`web/messages/en.json` (plate, clés à points, placeholders `{param}`,
compilée par [Paraglide JS](https://paraglidejs.com) en fonctions de
message élaguées sous `web/src/lib/paraglide/`, générée et non versionnée).
La traduction se fait sur
[hosted.weblate.org/projects/dumbmonit](https://hosted.weblate.org/projects/dumbmonit/)
(composant `web-ui`), configuré ainsi :

| Réglage | Valeur |
| --- | --- |
| Dépôt | ce dépôt, sous-dossier `web/` |
| Masque de fichier | `web/messages/*.json` |
| Fichier de langue de base monolingue | `web/messages/en.json` |
| Format de fichier | Fichier JSON (monolingue) |
| Nouvelle langue | crée un nouveau fichier de traduction (Weblate écrit `web/messages/<locale>.json`) |
| Contrôle de version | GitHub, via l'application Weblate GitHub : les traductions reviennent comme pull requests sur `main` |

Les nouvelles chaînes d'UI n'arrivent que dans `web/messages/en.json` ; un
traducteur remplit ensuite les autres fichiers de locale via Weblate. Les
clés sont extraites progressivement — aujourd'hui cela couvre les pages de
Réglages (`web/src/routes/settings/`) et la copie intégrée de quelques
primitives `web/src/lib/ui/` — le reste de l'UI lit encore de l'anglais
brut et est extrait au fur et à mesure qu'il est touché. Voir
`CONTRIBUTING.md` pour la convention d'écriture des chaînes.

`web/project.inlang/settings.json` pointe son seul plugin
(message-format) vers `./node_modules/@inlang/plugin-message-format/dist/index.js`
plutôt que l'URL jsdelivr que l'outillage propre d'inlang utilise par
défaut : un chemin relatif, non `http`, est lu directement depuis le
disque, donc `npm run build` ne contacte jamais un CDN — important
puisque la compilation de l'image n'a aucune garantie d'un cache npm
chaud offrant cette URL, et un outil homelab qui a besoin d'internet pour
se reconstruire fait mauvaise impression.

## Tests et CI

`.github/workflows/ci.yml` lance, à chaque push et pull request :

1. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features` ;
2. `cargo deny check licenses bans` — seules les licences approuvées OSI
   sont autorisées par `deny.toml` ; une dépendance copyleft ou
   source-available fait échouer la compilation ;
3. `npm run check` et `npm run build` ;
4. l'image (`linux/amd64`), compilation seulement.

`.github/workflows/release.yml` pousse la même image vers
`ghcr.io/laupernoe/dumbmonit` : `:edge` à chaque push sur `main`,
`:X.Y.Z`, `:X.Y`, `:X` et `:latest` à chaque tag `v*`.

Les tests d'intégration vivent dans `crates/server/tests/` ; les modules
purs (conversion de métriques, analyse d'options, machine à états
d'alerte, silences) sont testés en place sans réseau.

## Ajouter un collecteur

1. Implémentez `Collector` depuis `crates/proto/src/collector.rs` :
   `kind()`, `probe(&Target) -> Result<Vec<Sample>, ProbeError>` et
   optionnellement `discover()` pour la détection de profil. Renvoyez les
   variantes de `ProbeError` fidèlement : `means_down()` décide si un
   échec compte comme « appareil en panne » (alerté) ou comme une erreur
   de configuration (montrée sur l'appareil, non notifiée).
2. Lisez vos options depuis `Target::tags` dans un `options.rs`, avec des
   défauts.
3. Enregistrez-le dans `crates/server/src/main.rs` avec
   `registry.register(...)`. Le planificateur et l'API ne connaissent
   jamais les types concrets.
4. Décrivez-le dans `crates/server/src/api/collectors.rs` : label, résumé,
   exemples, types d'identifiant, indice d'adresse, avis de configuration
   et la liste `options` (clé, label, aide, placeholder, défaut, input,
   choix). L'UI construit son formulaire depuis ceci ; un test là-bas
   vérifie que chaque type enregistré a un avis et que les clés d'option
   correspondent à ce que le collecteur lit.
5. Nommez les métriques `<quoi>_<unité>` sans le préfixe `dumbmonit_`
   (l'écrivain l'ajoute) et ajoutez toute nouvelle métrique qu'une règle
   intégrée cible à la liste dans les tests de `alerting/rules.rs`.
6. Ajoutez une page sous `docs/devices/`.

Les échantillons sont `Sample::new(name, value, MetricKind::Gauge | Counter,
timestamp_ms)` avec `.with_label(k, v)` ; les labels d'identité (`target`,
`host`, `tag_*`) sont ajoutés par le registre.

## Ajouter un canal de notification

1. Implémentez `Notifier` (`crates/server/src/notify/mod.rs`) : `kind()`
   et `async fn send(&Message) -> Result<(), NotifyError>`. Les canaux
   web partagent le client HTTP et les aides dans `notify/http.rs` ;
   regardez `services.rs`, `chat.rs`, `push.rs`, `oncall.rs` et `smtp.rs`
   pour des exemples voisins.
2. Ajoutez le type à `CHANNEL_KINDS` et au match `build` dans
   `notify/mod.rs`, qui transforme une `ChannelConfig` stockée
   (réglages + secrets déchiffrés) en un `Notifier`.
3. Décrivez-le dans `notify/catalog.rs` : label, résumé, ancre `doc_url`,
   et les champs `settings` et `secrets` (clé, label, requis, input,
   aide, placeholder, options, forme, défaut). L'UI affiche le formulaire
   depuis cette description, aucune release d'UI n'est donc nécessaire.
4. Documentez-le dans `docs/notifications.md` sous *Canaux supportés*,
   avec un titre dont le slug correspond à l'ancre `doc_url` : l'UI
   pointe directement vers cette ancre sur Read the Docs.

## Captures d'écran headless

`web/tools/README.md` décrit comment rendre chaque page dans un Chromium
headless tournant dans Docker (`zenika/alpine-chrome:with-puppeteer`,
aucun navigateur local nécessaire) : se connecter avec `curl` pour
obtenir le cookie `dumbmonit_session`, puis lancer
`web/tools/screenshot.js` contre le serveur de prévisualisation ou
directement contre `:8080`. Ça écrit
`web/shots/<page>-<theme>-<viewport>.png` pour les deux thèmes et les deux
viewports, c'est comme ça que l'UI est vérifiée après un changement et
comme les captures de cette documentation ont été faites.

## Documentation

Ce site est construit avec MkDocs et Material for MkDocs depuis `docs/`
(`mkdocs.yml` à la racine, `.readthedocs.yaml` pour Read the Docs). Pour
prévisualiser :

```bash
pip install -r docs/requirements.txt
mkdocs serve
mkdocs build --strict   # ce que lance Read the Docs ; doit passer sans avertissement
```

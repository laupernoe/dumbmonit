# Entwicklung

DumbMonit ist ein Rust-Binary (`crates/server`, das den SvelteKit-Build
einbettet) plus VictoriaMetrics für Zeitreihen, vom Server aus dem im Image
mitgelieferten Binary gestartet, und eine eingebettete SQLite-Datenbank für
Konfiguration und Zustand. Ein Container. Rust-Code und Kommentare sind auf
Französisch; die Web-UI ist auf Englisch.

## Projektstruktur {#project-layout}

```
crates/proto     shared types: Sample, Target, Credential, trait Collector (+ ProbeError)
crates/server    the binary
  api/           axum routes (mod.rs is the route table); spa.rs serves web/build
  auth/          single instance password, HttpOnly session cookie, rate limit
  collectors/    snmp (profiles/*.yaml loaded at startup), proxmox, pbs, synology, agent, uptime
  scheduler.rs   runs every enabled target on its interval through the collector registry
  tsdb/          VictoriaMetrics writer (batched flush) + query proxy; embedded.rs supervises the child VictoriaMetrics
  db/            SQLite + numbered migrations in db/migrations/
  alerting/      rules, state machine (Phase/EffectivePhase), suppression by parent, silences, seasonal baseline
  notify/        22 notification channels, described to the UI by notify/catalog.rs
  crypto.rs      AES-256-GCM for credentials/tokens, key derived from /data/secret.key or DUMBMONIT_SECRET
crates/agent     Linux/macOS/FreeBSD/Windows agent; install/ holds install.sh/.ps1 served by the server
web/             SvelteKit (Svelte 5 runes, Tailwind 4, uPlot), adapter-static SPA fallback, ssr=false
profiles/        SNMP collection profiles, auto-applied by sysObjectID
docs/            this documentation (MkDocs, published on Read the Docs)
```

## Ohne Rust-Toolchain bauen {#building-without-a-rust-toolchain}

Die Kompilierung läuft in Docker; auf dem Host muss nichts Rust-Bezogenes
installiert werden.

```bash
# Full stack (builds the image: about 10 min cold) — UI on http://localhost:8080
docker compose up -d --build
# developer overlay: the embedded VictoriaMetrics is published on :8428 for direct MetricsQL queries
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build

# Rust, inside the builder stage (musl/alpine)
docker build -t dumbmonit-devenv --target builder .
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server --test alerts_api   # one integration test file
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server test_name          # one test by name
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo clippy --all-targets --all-features -- -D warnings
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo fmt --all --check
```

Mit lokal installiertem Rust funktionieren `cargo test` und
`cargo clippy -- -D warnings` direkt. Edition 2024, MSRV 1.88;
`rustfmt.toml` setzt `max_width = 100` und `use_small_heuristics = "Max"`.

Der Docker-Build ist geschichtet: Web-Build → Rust-Abhängigkeiten auf
Dummy-Quellen → echte Quellen (mit einem `touch` vor dem zweiten
`cargo build`, sonst nutzt cargo die Dummy-Artefakte weiter) → Agent
cross-compiliert mit cargo-zigbuild für `x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-musl` und `x86_64-pc-windows-gnu` → `FROM scratch`.
`web/build` muss vor `cargo build` existieren, sonst liefert der Server ohne
UI.

## Web-UI {#web-ui}

Node 22 reicht auf dem Host.

```bash
cd web && npm run dev        # Vite on :5173, proxies /api to localhost:8080 (the Docker server)
cd web && npm run check      # svelte-check (CI runs this)
cd web && npm run build      # static build into web/build, embedded by rust_embed at cargo build
```

Konventionen: Nur Svelte-5-Runes (`$props`, `$state`, `$derived`,
`$effect`, Snippets, `onclick`). Jeder Netzwerkzugriff läuft über
`src/lib/api/`; Komponenten rufen nie `fetch` auf. Typen in `api/types.ts`
spiegeln die Rust-Structs exakt und müssen mit
`crates/server/src/api/*.rs` synchron gehalten werden. Das Designsystem
(Tokens in `src/app.css`, Primitive in `src/lib/ui/`) ist in `DESIGN.md`
dokumentiert; der Produktkontext in `PRODUCT.md`. Nie zwei `vite build`
gleichzeitig laufen lassen: sie löschen `.svelte-kit/output`.

## Übersetzungen (Weblate) {#translations-weblate}

Die Web-UI ist für Community-Übersetzung vorbereitet, aber es ist noch
nichts übersetzt: Englisch ist die einzige mitgelieferte Sprache, in
`web/messages/en.json` (flach, Schlüssel mit Punkten, `{param}`-Platzhalter,
von [Paraglide JS](https://paraglidejs.com) in tree-geshakte Message-
Funktionen unter `web/src/lib/paraglide/` kompiliert, generiert und nicht
versioniert). Übersetzung findet auf
[hosted.weblate.org/projects/dumbmonit](https://hosted.weblate.org/projects/dumbmonit/)
statt (Komponente `web-ui`), konfiguriert wie folgt:

| Einstellung | Wert |
| --- | --- |
| Repository | dieses Repo, Unterverzeichnis `web/` |
| File mask | `web/messages/*.json` |
| Monolingual base language file | `web/messages/en.json` |
| File format | JSON file (monolingual) |
| New language | eine neue Übersetzungsdatei erstellen (Weblate schreibt `web/messages/<locale>.json`) |
| Version control | GitHub, über die Weblate-GitHub-App: Übersetzungen kommen als Pull Requests auf `main` zurück |

Neue UI-Strings landen nur in `web/messages/en.json`; ein Übersetzer füllt
dann die anderen Sprachdateien über Weblate. Schlüssel werden schrittweise
extrahiert — heute deckt das die Settings-Seiten
(`web/src/routes/settings/`) und die eingebaute Kopie einiger
`web/src/lib/ui/`-Primitive ab — der Rest der UI liest sich noch als reines
Englisch und wird extrahiert, sobald er angefasst wird. Siehe
`CONTRIBUTING.md` für die Konvention zum Verfassen von Strings.

`web/project.inlang/settings.json` verweist sein einziges Plugin
(message-format) auf `./node_modules/@inlang/plugin-message-format/dist/index.js`
statt auf die jsdelivr-URL, auf die inlangs eigenes Tooling standardmäßig
zeigt: Ein relativer, nicht-`http`-Pfad wird direkt von der Platte
gelesen, sodass `npm run build` nie ein CDN kontaktiert — wichtig, da der
Image-Build keine Garantie für einen warmen npm-Cache mit dieser URL hat,
und ein Homelab-Tool, das das Internet braucht, um sich selbst neu zu
bauen, ist keine gute Optik.

## Tests und CI {#tests-and-ci}

`.github/workflows/ci.yml` läuft bei jedem Push und Pull Request:

1. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features`;
2. `cargo deny check licenses bans` — nur OSI-genehmigte Lizenzen sind
   nach `deny.toml` erlaubt; eine Copyleft- oder Source-Available-
   Abhängigkeit lässt den Build fehlschlagen;
3. `npm run check` und `npm run build`;
4. das Image (`linux/amd64`), nur Build.

`.github/workflows/release.yml` pusht dasselbe Image nach
`ghcr.io/laupernoe/dumbmonit`: `:edge` bei jedem Push nach `main`,
`:X.Y.Z`, `:X.Y`, `:X` und `:latest` bei jedem `v*`-Tag.

Integrationstests liegen in `crates/server/tests/`; reine Module
(Metrik-Konvertierung, Options-Parsing, Alarm-Zustandsmaschine, Silences)
werden ohne Netzwerk direkt getestet.

## Einen Collector hinzufügen {#adding-a-collector}

1. `Collector` aus `crates/proto/src/collector.rs` implementieren:
   `kind()`, `probe(&Target) -> Result<Vec<Sample>, ProbeError>` und
   optional `discover()` zur Profilerkennung. `ProbeError`-Varianten
   wahrheitsgemäß zurückgeben: `means_down()` entscheidet, ob ein Fehler
   als "Gerät down" zählt (alarmiert) oder als Konfigurationsfehler (am
   Gerät angezeigt, nicht benachrichtigt).
2. Eigene Optionen aus `Target::tags` in einer `options.rs` lesen, mit
   Standardwerten.
3. In `crates/server/src/main.rs` mit `registry.register(...)`
   registrieren. Der Scheduler und die API kennen nie konkrete Typen.
4. In `crates/server/src/api/collectors.rs` beschreiben: Label,
   Zusammenfassung, Beispiele, Credential-Typen, Adresshinweis,
   Einrichtungshinweis und die `options`-Liste (Schlüssel, Label, Hilfe,
   Platzhalter, Standard, Input, Auswahlmöglichkeiten). Die UI baut ihr
   Formular hieraus; ein Test dort prüft, dass jede registrierte Art einen
   Hinweis hat und dass die Options-Schlüssel dem entsprechen, was der
   Collector liest.
5. Metriken `<was>_<einheit>` ohne das `dumbmonit_`-Präfix benennen (der
   Writer fügt es hinzu) und jede neue Metrik, auf die eine eingebaute
   Regel zielt, zur Liste in den `alerting/rules.rs`-Tests hinzufügen.
6. Eine Seite unter `docs/devices/` hinzufügen.

Samples sind `Sample::new(name, value, MetricKind::Gauge | Counter,
timestamp_ms)` mit `.with_label(k, v)`; Identitätslabels (`target`, `host`,
`tag_*`) werden von der Registry hinzugefügt.

## Einen Benachrichtigungskanal hinzufügen {#adding-a-notification-channel}

1. `Notifier` (`crates/server/src/notify/mod.rs`) implementieren:
   `kind()` und `async fn send(&Message) -> Result<(), NotifyError>`.
   Web-Kanäle teilen den HTTP-Client und die Helfer in
   `notify/http.rs`; siehe `services.rs`, `chat.rs`, `push.rs`,
   `oncall.rs` und `smtp.rs` für verwandte Beispiele.
2. Die Art zu `CHANNEL_KINDS` und zum `build`-Match in `notify/mod.rs`
   hinzufügen, der eine gespeicherte `ChannelConfig` (Settings +
   entschlüsselte Secrets) in einen `Notifier` verwandelt.
3. In `notify/catalog.rs` beschreiben: Label, Zusammenfassung,
   `doc_url`-Anker, und die Felder `settings` und `secrets` (Schlüssel,
   Label, erforderlich, Input, Hilfe, Platzhalter, Optionen, Form,
   Standard). Die UI rendert das Formular aus dieser Beschreibung, daher
   ist kein UI-Release nötig.
4. In `docs/notifications.md` unter *Supported channels* dokumentieren,
   mit einer Überschrift, deren Slug zum `doc_url`-Anker passt: Die UI
   verlinkt direkt auf diesen Anker auf Read the Docs.

## Headless-Screenshots {#headless-screenshots}

`web/tools/README.md` beschreibt, wie jede Seite in einem Headless-Chromium
in Docker (`zenika/alpine-chrome:with-puppeteer`, kein lokaler Browser
nötig) gerendert wird: mit `curl` einloggen, um das `dumbmonit_session`-
Cookie zu erhalten, dann `web/tools/screenshot.js` gegen den Preview-Server
oder direkt gegen `:8080` laufen lassen. Es schreibt
`web/shots/<page>-<theme>-<viewport>.png` für beide Themes und beide
Viewports, womit die UI nach einer Änderung geprüft wird und wie die
Screenshots dieser Dokumentation entstanden sind.

## Dokumentation {#documentation}

Diese Website wird mit MkDocs und Material for MkDocs aus `docs/` gebaut
(`mkdocs.yml` im Root, `.readthedocs.yaml` für Read the Docs). Zur Vorschau:

```bash
pip install -r docs/requirements.txt
mkdocs serve
mkdocs build --strict   # what Read the Docs runs; must pass with no warnings
```

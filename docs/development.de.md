# Entwicklung {#development}

DumbMonit besteht aus einem Rust-Binary (`crates/server`, das den SvelteKit-Build einbettet),
VictoriaMetrics für Zeitreihen, vom Server aus dem im Image mitgelieferten Binary gestartet,
und einer eingebetteten SQLite-Datenbank für Konfiguration und Zustand. Ein Container. Rust-Code
und Kommentare sind auf Französisch; die Web-Oberfläche ist auf Englisch.

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

## Bauen ohne Rust-Toolchain {#building-without-a-rust-toolchain}

Die Kompilierung läuft in Docker; auf dem Host muss nichts von Rust installiert sein.

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

Mit lokal installiertem Rust funktionieren `cargo test` und `cargo clippy -- -D warnings`
direkt. Edition 2024, MSRV 1.88; `rustfmt.toml` setzt `max_width = 100` und
`use_small_heuristics = "Max"`.

Der Docker-Build ist in Schichten aufgebaut: Web-Build → Rust-Abhängigkeiten auf Dummy-Quellen →
echte Quellen (mit einem `touch` vor dem zweiten `cargo build`, sonst verwendet cargo die
Dummy-Artefakte weiter) → Agent, mit cargo-zigbuild cross-kompiliert für
`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` und `x86_64-pc-windows-gnu` →
`FROM scratch`. `web/build` muss vor `cargo build` existieren, sonst wird der Server ohne
Oberfläche ausgeliefert.

## Web-Oberfläche {#web-ui}

Auf dem Host genügt Node 22.

```bash
cd web && npm run dev        # Vite on :5173, proxies /api to localhost:8080 (the Docker server)
cd web && npm run check      # svelte-check (CI runs this)
cd web && npm run build      # static build into web/build, embedded by rust_embed at cargo build
```

Konventionen: ausschließlich Svelte-5-Runes (`$props`, `$state`, `$derived`, `$effect`,
Snippets, `onclick`). Der gesamte Netzwerkzugriff läuft über `src/lib/api/`; Komponenten rufen
nie `fetch` auf. Die Typen in `api/types.ts` spiegeln die Rust-Structs exakt und müssen mit
`crates/server/src/api/*.rs` synchron gehalten werden. Das Designsystem liegt in `src/app.css`
(Tokens) und `src/lib/ui/` (Primitive). Führen Sie nie zwei `vite build` gleichzeitig aus: Sie
löschen `.svelte-kit/output`.

## Übersetzungen (Weblate) {#translations-weblate}

Die Web-Oberfläche ist für Community-Übersetzungen vorbereitet, aber noch nichts ist übersetzt:
Englisch ist die einzige ausgelieferte Sprache, in `web/messages/en.json` (flache Schlüssel mit
Punkten, Platzhalter `{param}`, von [Paraglide JS](https://paraglidejs.com) in Tree-Shaking-fähige
Nachrichtenfunktionen unter `web/src/lib/paraglide/` kompiliert, generiert und nicht
versioniert). Die Übersetzung erfolgt auf
[hosted.weblate.org/projects/dumbmonit](https://hosted.weblate.org/projects/dumbmonit/)
(Komponente `web-ui`), wie folgt konfiguriert:

| Einstellung | Wert |
| --- | --- |
| Repository | dieses Repo, Unterverzeichnis `web/` |
| Dateimaske | `web/messages/*.json` |
| Monolinguale Basissprachdatei | `web/messages/en.json` |
| Dateiformat | JSON-Datei (monolingual) |
| Neue Sprache | neue Übersetzungsdatei anlegen (Weblate schreibt `web/messages/<locale>.json`) |
| Versionskontrolle | GitHub, über die Weblate-GitHub-App: Übersetzungen kommen als Pull Requests auf `main` zurück |

Neue UI-Texte landen nur in `web/messages/en.json`; ein Übersetzer füllt anschließend die
übrigen Sprachdateien über Weblate. Die Schlüssel werden schrittweise extrahiert — derzeit
betrifft das die Einstellungsseiten (`web/src/routes/settings/`) und den eingebauten Text
einiger Primitive in `web/src/lib/ui/` — der Rest der Oberfläche liest noch einfaches Englisch
und wird extrahiert, sobald er angefasst wird. Die Konvention zum Verfassen von Texten steht in
`CONTRIBUTING.md`.

`web/project.inlang/settings.json` verweist das einzige Plugin (message-format) auf
`./node_modules/@inlang/plugin-message-format/dist/index.js` statt auf die jsdelivr-URL, die
das Tooling von inlang standardmäßig verwendet: Ein relativer Nicht-`http`-Pfad wird direkt von
der Festplatte gelesen, sodass `npm run build` nie ein CDN kontaktiert — wichtig, weil der
Image-Build keinen warmen npm-Cache für diese URL garantiert, und ein Homelab-Tool, das zum
eigenen Neubau das Internet braucht, macht keine gute Figur.

## Tests und CI {#tests-and-ci}

`.github/workflows/ci.yml` führt bei jedem Push und Pull Request aus:

1. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features`;
2. `cargo deny check licenses bans` — `deny.toml` erlaubt nur OSI-zugelassene Lizenzen; eine
   Copyleft- oder Source-available-Abhängigkeit lässt den Build fehlschlagen;
3. `npm run check` und `npm run build`;
4. das Image (`linux/amd64`), nur Build.

`.github/workflows/release.yml` pusht dasselbe Image nach `ghcr.io/laupernoe/dumbmonit`:
`:edge` bei jedem Push auf `main`, `:X.Y.Z`, `:X.Y`, `:X` und `:latest` bei jedem `v*`-Tag.

Integrationstests liegen in `crates/server/tests/`; reine Module (Metrikumwandlung,
Optionen-Parsing, Alarm-Zustandsmaschine, Stummschaltungen) werden vor Ort ohne Netzwerk getestet.

## Einen Collector hinzufügen {#adding-a-collector}

1. Implementieren Sie `Collector` aus `crates/proto/src/collector.rs`: `kind()`,
   `probe(&Target) -> Result<Vec<Sample>, ProbeError>` und optional `discover()` für die
   Profilerkennung. Geben Sie die `ProbeError`-Varianten wahrheitsgemäß zurück: `means_down()`
   entscheidet, ob ein Fehler als „Gerät ausgefallen“ (alarmiert) oder als Konfigurationsfehler
   (am Gerät angezeigt, nicht benachrichtigt) zählt.
2. Lesen Sie Ihre Optionen aus `Target::tags` in einer `options.rs`, mit Standardwerten.
3. Registrieren Sie ihn in `crates/server/src/main.rs` mit `registry.register(...)`. Scheduler
   und API kennen nie konkrete Typen.
4. Beschreiben Sie ihn in `crates/server/src/api/collectors.rs`: Label, Zusammenfassung,
   Beispiele, Credential-Typen, Adresshinweis, Einrichtungshinweis und die Liste `options`
   (key, label, help, placeholder, default, input, choices). Die Oberfläche baut daraus ihr
   Formular; ein Test dort prüft, dass jede registrierte Art einen Hinweis hat und dass die
   Optionsschlüssel dem entsprechen, was der Collector liest.
5. Benennen Sie Metriken `<what>_<unit>` ohne das Präfix `dumbmonit_` (der Writer fügt es
   hinzu) und tragen Sie jede neue Metrik, auf die eine eingebaute Regel zielt, in die Liste in
   den Tests von `alerting/rules.rs` ein.
6. Fügen Sie unter `docs/devices/` eine Seite hinzu.

Samples sind `Sample::new(name, value, MetricKind::Gauge | Counter, timestamp_ms)` mit
`.with_label(k, v)`; Identitätslabels (`target`, `host`, `tag_*`) fügt die Registry hinzu.
Geben Sie sie nicht selbst aus: Benennen Sie ein Label pro Objekt nach dem Objekt
(`proxy_host`, `esxi`, `ceph_host`). Ein kollidierendes Label bleibt als `exported_<name>`
erhalten, und die für die Art in `api/collectors.rs` aufgeführten Optionen werden nicht als
`tag_*` kopiert.

## Einen Benachrichtigungskanal hinzufügen {#adding-a-notification-channel}

1. Implementieren Sie `Notifier` (`crates/server/src/notify/mod.rs`): `kind()` und
   `async fn send(&Message) -> Result<(), NotifyError>`. Web-Kanäle teilen sich den HTTP-Client
   und die Helfer in `notify/http.rs`; als Geschwister dienen `services.rs`, `chat.rs`,
   `push.rs`, `oncall.rs` und `smtp.rs`.
2. Fügen Sie die Art zu `CHANNEL_KINDS` und zum `build`-Match in `notify/mod.rs` hinzu, das
   eine gespeicherte `ChannelConfig` (Einstellungen + entschlüsselte Geheimnisse) in einen
   `Notifier` verwandelt.
3. Beschreiben Sie ihn in `notify/catalog.rs`: Label, Zusammenfassung, `doc_url`-Anker sowie die
   Felder `settings` und `secrets` (key, label, required, input, help, placeholder, options,
   shape, default). Die Oberfläche rendert das Formular aus dieser Beschreibung, daher ist kein
   UI-Release nötig.
4. Dokumentieren Sie ihn in `docs/notifications.md` unter *Unterstützte Kanäle*, mit einer
   Überschrift, deren Slug zum `doc_url`-Anker passt: Die Oberfläche verlinkt direkt auf diesen
   Anker bei Read the Docs.

## Headless-Screenshots {#headless-screenshots}

`web/tools/README.md` beschreibt, wie jede Seite in einem headless Chromium gerendert wird, der
in Docker läuft (`zenika/alpine-chrome:with-puppeteer`, kein lokaler Browser nötig): Mit `curl`
anmelden, um das Cookie `dumbmonit_session` zu erhalten, dann `web/tools/screenshot.js` gegen
den Preview-Server oder direkt gegen `:8080` ausführen. Es schreibt
`web/shots/<page>-<theme>-<viewport>.png` für beide Themes und beide Viewports; so wird die
Oberfläche nach einer Änderung geprüft und so entstanden die Screenshots dieser Dokumentation.

## Dokumentation {#documentation}

Diese Website wird mit MkDocs und Material for MkDocs aus `docs/` gebaut (`mkdocs.yml` im
Wurzelverzeichnis, `.readthedocs.yaml` für Read the Docs). Zur Vorschau:

```bash
pip install -r docs/requirements.txt
mkdocs serve
mkdocs build --strict   # what Read the Docs runs; must pass with no warnings
```

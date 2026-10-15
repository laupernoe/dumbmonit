# Desarrollo {#development}

DumbMonit es un único binario Rust (`crates/server`, que incluye la
compilación de SvelteKit) más VictoriaMetrics para series temporales,
iniciado por el servidor desde el binario incluido en la imagen, y una
base de datos SQLite integrada para configuración y estado. Un
contenedor. El código Rust y los comentarios están en francés; la
interfaz web está en inglés.

## Estructura del proyecto {#project-layout}

```
crates/proto     tipos compartidos: Sample, Target, Credential, trait Collector (+ ProbeError)
crates/server    el binario
  api/           rutas axum (mod.rs es la tabla de rutas); spa.rs sirve web/build
  auth/          contraseña de instancia única, cookie de sesión HttpOnly, limitación de tasa
  collectors/    snmp (profiles/*.yaml cargados al arrancar), proxmox, pbs, synology, agent, uptime
  scheduler.rs   ejecuta cada target activado en su intervalo a través del registro de recolectores
  tsdb/          escritor de VictoriaMetrics (flush agrupado) + proxy de consultas; embedded.rs supervisa el VictoriaMetrics hijo
  db/            SQLite + migraciones numeradas en db/migrations/
  alerting/      reglas, máquina de estados (Phase/EffectivePhase), supresión por padre, silencios, referencia estacional
  notify/        22 canales de notificación, descritos a la UI por notify/catalog.rs
  crypto.rs      AES-256-GCM para credenciales/tokens, clave derivada de /data/secret.key o DUMBMONIT_SECRET
crates/agent     agente Linux/macOS/FreeBSD/Windows; install/ contiene install.sh/.ps1 servidos por el servidor
web/             SvelteKit (Svelte 5 runes, Tailwind 4, uPlot), fallback SPA adapter-static, ssr=false
profiles/        perfiles de recolección SNMP, aplicados automáticamente por sysObjectID
docs/            esta documentación (MkDocs, publicada en Read the Docs)
```

## Compilar sin un toolchain de Rust {#building-without-a-rust-toolchain}

La compilación ocurre en Docker; nada de Rust necesita instalarse en el
host.

```bash
# Stack completo (construye la imagen: unos 10 min en frío) — UI en http://localhost:8080
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build

# Rust, dentro de la etapa builder (musl/alpine)
docker build -t dumbmonit-devenv --target builder .
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server --test alerts_api   # un archivo de test de integración
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server test_name          # un test por nombre
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo clippy --all-targets --all-features -- -D warnings
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo fmt --all --check
```

Con Rust instalado localmente, `cargo test` y `cargo clippy -- -D
warnings` funcionan directamente. Edition 2024, MSRV 1.95;
`rustfmt.toml` define `max_width = 100` y `use_small_heuristics = "Max"`.

La compilación Docker está en capas: compilación web → dependencias Rust
sobre fuentes ficticias → fuentes reales (con un `touch` antes del
segundo `cargo build`, de lo contrario cargo reutiliza los artefactos
ficticios) → agente compilado de forma cruzada con cargo-zigbuild para
`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` y
`x86_64-pc-windows-gnu` → `FROM scratch`. `web/build` debe existir antes
de `cargo build`, o el servidor se distribuye sin interfaz.

## Interfaz web {#web-ui}

Node 22 basta en el host.

```bash
cd web && npm run dev        # Vite en :5173, redirige /api a localhost:8080 (el servidor Docker)
cd web && npm run check      # svelte-check (CI lo ejecuta)
cd web && npm run build      # compilación estática en web/build, incluida por rust_embed en cargo build
```

Convenciones: solo Svelte 5 runes (`$props`, `$state`, `$derived`,
`$effect`, snippets, `onclick`). Todo el acceso a red pasa por
`src/lib/api/`; los componentes nunca hacen `fetch`. Los tipos en
`api/types.ts` reflejan exactamente las estructuras Rust y deben
mantenerse sincronizados con `crates/server/src/api/*.rs`. El sistema de
diseño vive en `src/app.css` (tokens) y `src/lib/ui/` (primitivas).
Nunca ejecutes dos `vite build` a la vez: borran `.svelte-kit/output`.

## Traducciones {#translations}

La interfaz web se traduce mediante pull request, como cualquier otra contribución. El idioma de origen es el inglés (`web/messages/en.json`: claves planas, marcadores `{param}`); cada otro idioma es un archivo, `web/messages/<locale>.json`, que [Paraglide JS](https://paraglidejs.com) compila en funciones de mensaje con tree-shaking bajo `web/src/lib/paraglide/` (generado, sin seguimiento).

1. Edita `web/messages/<locale>.json` (`fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans`), o copia `en.json` para añadir un idioma nuevo.
2. Conserva todas las claves y todas las `{variables}` de `en.json`; traduce solo los valores. Sin marcado HTML en los mensajes.
3. Los plurales usan la sintaxis de mensajes, no cadenas armadas a mano: conserva la estructura `match`/`plural` del mensaje en inglés y traduce cada rama.
4. Ejecuta `cd web && npm run i18n:check`: falla si a un idioma le falta una clave o cambia sus `{variables}`.
5. Abre un pull request.

Las cadenas nuevas de la interfaz se añaden primero a `web/messages/en.json` (el único archivo que necesita un pull request de funcionalidad; `npm run i18n:merge` fusiona fragmentos temporales). Un idioma aparece en el selector (Ajustes, Apariencia) en cuanto existe su archivo. La convención de nombres de claves está en `CONTRIBUTING.md`.

`web/project.inlang/settings.json` apunta su único plugin
(message-format) a `./node_modules/@inlang/plugin-message-format/dist/index.js`
en lugar de la URL de jsdelivr a la que apunta por defecto el propio
tooling de inlang: una ruta relativa, no `http`, se lee directamente del
disco, así que `npm run build` nunca contacta a un CDN — importante ya
que la compilación de la imagen no tiene ninguna garantía de una caché
npm caliente que ofrezca esa URL, y una herramienta de homelab que
necesita internet para reconstruirse es una mala señal.

## Tests y CI {#tests-and-ci}

`.github/workflows/ci.yml` ejecuta, en cada push y pull request:

1. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features
   -- -D warnings`, `cargo test --all-features`;
2. `cargo deny check licenses bans` — solo se permiten licencias
   aprobadas por la OSI según `deny.toml`; una dependencia copyleft o
   source-available hace fallar la compilación;
3. `npm run check` y `npm run build`;
4. la imagen (`linux/amd64`), solo compilación.

`.github/workflows/release.yml` publica la misma imagen en
`ghcr.io/laupernoe/dumbmonit`: `:edge` en cada push a `main`, `:X.Y.Z`,
`:X.Y`, `:X` y `:latest` en cada etiqueta `v*`.

Los tests de integración viven en `crates/server/tests/`; los módulos
puros (conversión de métricas, análisis de opciones, máquina de estados
de alertas, silencios) se prueban en el sitio sin red.

## Añadir un recolector {#adding-a-collector}

1. Implementa `Collector` desde `crates/proto/src/collector.rs`:
   `kind()`, `probe(&Target) -> Result<Vec<Sample>, ProbeError>` y
   opcionalmente `discover()` para la detección de perfil. Devuelve las
   variantes de `ProbeError` con veracidad: `means_down()` decide si un
   fallo cuenta como «monitor caído» (alertado) o como un error de
   configuración (mostrado en el monitor, no notificado).
2. Lee tus opciones de `Target::tags` en un `options.rs`, con valores por
   defecto.
3. Regístralo en `crates/server/src/main.rs` con
   `registry.register(...)`. El scheduler y la API nunca conocen tipos
   concretos.
4. Descríbelo en `crates/server/src/api/collectors.rs`: etiqueta,
   resumen, ejemplos, tipos de credencial, pista de dirección, aviso de
   configuración y la lista de `options` (clave, etiqueta, ayuda,
   placeholder, por defecto, input, opciones). La UI construye su
   formulario a partir de esto; un test ahí comprueba que cada tipo
   registrado tiene un aviso y que las claves de opción coinciden con lo
   que lee el recolector.
5. Nombra las métricas `<qué>_<unidad>` sin el prefijo `dumbmonit_` (el
   escritor lo añade) y añade cualquier métrica nueva que una regla
   integrada apunte a la lista en los tests de `alerting/rules.rs`.
6. Añade una página bajo `docs/devices/`.

Las muestras son `Sample::new(name, value, MetricKind::Gauge | Counter,
timestamp_ms)` con `.with_label(k, v)`; las etiquetas de identidad
(`target`, `host`, `tag_*`) las añade el registro. No las emitas tú: nombra una
etiqueta por objeto con el nombre del objeto (`proxy_host`, `esxi`, `ceph_host`).
Una etiqueta que colisiona se conserva como `exported_<name>`, y las opciones
listadas para el tipo en `api/collectors.rs` no se copian como `tag_*`.

## Añadir un canal de notificación {#adding-a-notification-channel}

1. Implementa `Notifier` (`crates/server/src/notify/mod.rs`): `kind()` y
   `async fn send(&Message) -> Result<(), NotifyError>`. Los canales web
   comparten el cliente HTTP y los helpers de `notify/http.rs`; mira
   `services.rs`, `chat.rs`, `push.rs`, `oncall.rs` y `smtp.rs` como
   referencia.
2. Añade el tipo a `CHANNEL_KINDS` y al match `build` en
   `notify/mod.rs`, que convierte un `ChannelConfig` guardado (ajustes +
   secretos descifrados) en un `Notifier`.
3. Descríbelo en `notify/catalog.rs`: etiqueta, resumen, ancla `doc_url`,
   y los campos `settings` y `secrets` (clave, etiqueta, obligatorio,
   input, ayuda, placeholder, opciones, forma, por defecto). La UI
   renderiza el formulario a partir de esta descripción, así que no se
   necesita ninguna versión de UI.
4. Documéntalo en `docs/notifications.md` bajo *Supported channels*, con
   un encabezado cuyo slug coincida con el ancla `doc_url`: la UI enlaza
   directamente a ese ancla en Read the Docs.

## Capturas sin interfaz {#headless-screenshots}

`web/tools/README.md` describe cómo renderizar cada página en un
Chromium sin interfaz ejecutándose en Docker
(`zenika/alpine-chrome:with-puppeteer`, sin necesidad de navegador
local): inicia sesión con `curl` para obtener la cookie
`dumbmonit_session`, y luego ejecuta `web/tools/screenshot.js` contra el
servidor de vista previa o directamente contra `:8080`. Escribe
`web/shots/<page>-<theme>-<viewport>.png` para ambos temas y ambos
viewports, que es cómo se comprueba la UI tras un cambio y cómo se
hicieron las capturas de esta documentación.

## Documentación {#documentation}

Este sitio se construye con MkDocs y Material for MkDocs desde `docs/`
(`mkdocs.yml` en la raíz, `.readthedocs.yaml` para Read the Docs). Para
previsualizar:

```bash
pip install -r docs/requirements.txt
mkdocs serve
mkdocs build --strict   # lo que ejecuta Read the Docs; debe pasar sin ningún aviso
```

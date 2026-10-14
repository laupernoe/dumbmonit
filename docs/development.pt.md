# Desenvolvimento {#development}

O DumbMonit é um binário Rust (`crates/server`, que incorpora a build do
SvelteKit) mais o VictoriaMetrics para séries temporais, iniciado pelo servidor
a partir do binário fornecido na imagem, e uma base de dados SQLite incorporada
para configuração e estado. Um contentor. O código Rust e os comentários estão
em francês; a interface web está em inglês.

## Estrutura do projeto {#project-layout}

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

## Compilar sem toolchain Rust {#building-without-a-rust-toolchain}

A compilação faz-se no Docker; não é preciso instalar nada de Rust no anfitrião.

```bash
# Full stack (builds the image: about 10 min cold) — UI on http://localhost:8080
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build

# Rust, inside the builder stage (musl/alpine)
docker build -t dumbmonit-devenv --target builder .
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server --test alerts_api   # one integration test file
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server test_name          # one test by name
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo clippy --all-targets --all-features -- -D warnings
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo fmt --all --check
```

Com o Rust instalado localmente, `cargo test` e `cargo clippy -- -D warnings`
funcionam diretamente. Edition 2024, MSRV 1.95; o `rustfmt.toml` define
`max_width = 100` e `use_small_heuristics = "Max"`.

A build Docker é feita por camadas: build web → dependências Rust sobre fontes
fictícias → fontes reais (com um `touch` antes do segundo `cargo build`, caso
contrário o cargo reutiliza os artefactos fictícios) → agente compilado em
cruzado com cargo-zigbuild para `x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-musl` e `x86_64-pc-windows-gnu` → `FROM scratch`.
`web/build` tem de existir antes do `cargo build`, ou o servidor é entregue sem
interface.

## Interface web {#web-ui}

O Node 22 chega no anfitrião.

```bash
cd web && npm run dev        # Vite on :5173, proxies /api to localhost:8080 (the Docker server)
cd web && npm run check      # svelte-check (CI runs this)
cd web && npm run build      # static build into web/build, embedded by rust_embed at cargo build
```

Convenções: apenas runes do Svelte 5 (`$props`, `$state`, `$derived`,
`$effect`, snippets, `onclick`). Todo o acesso à rede passa por `src/lib/api/`;
os componentes nunca fazem `fetch`. Os tipos em `api/types.ts` espelham
exatamente as structs Rust e têm de ser mantidos em sincronia com
`crates/server/src/api/*.rs`. O sistema de design vive em `src/app.css`
(tokens) e `src/lib/ui/` (primitivas). Nunca execute dois `vite build` ao mesmo
tempo: apagam `.svelte-kit/output`.

## Traduções (Weblate) {#translations-weblate}

A interface web está preparada para tradução comunitária mas nada está ainda
traduzido: o inglês é o único idioma fornecido, em `web/messages/en.json`
(chaves planas, com pontos, marcadores `{param}`, compiladas pelo
[Paraglide JS](https://paraglidejs.com) em funções de mensagem com tree-shaking
em `web/src/lib/paraglide/`, geradas e não versionadas). A tradução faz-se em
[hosted.weblate.org/projects/dumbmonit](https://hosted.weblate.org/projects/dumbmonit/)
(componente `web-ui`), configurado da seguinte forma:

| Definição | Valor |
| --- | --- |
| Repositório | este repositório, subdiretoria `web/` |
| Máscara de ficheiro | `web/messages/*.json` |
| Ficheiro de idioma base monolingue | `web/messages/en.json` |
| Formato de ficheiro | Ficheiro JSON (monolingue) |
| Novo idioma | criar um novo ficheiro de tradução (o Weblate escreve `web/messages/<locale>.json`) |
| Controlo de versões | GitHub, através da app GitHub do Weblate: as traduções voltam como pull requests em `main` |

As novas cadeias da interface entram apenas em `web/messages/en.json`; um
tradutor preenche depois os outros ficheiros de idioma através do Weblate. As
chaves são extraídas progressivamente — hoje isso cobre as páginas de Definições
(`web/src/routes/settings/`) e o texto incorporado de algumas primitivas de
`web/src/lib/ui/` — o resto da interface ainda lê inglês simples e é extraído
à medida que é tocado. Veja o `CONTRIBUTING.md` para a convenção de escrita de
cadeias.

O `web/project.inlang/settings.json` aponta o seu único plugin (message-format)
para `./node_modules/@inlang/plugin-message-format/dist/index.js` em vez do URL
jsdelivr que as próprias ferramentas do inlang usam por predefinição: um
caminho relativo, não `http`, é lido diretamente do disco, pelo que
`npm run build` nunca vai a uma CDN — importante porque a build da imagem não
garante uma cache npm quente que ofereça esse URL, e uma ferramenta de homelab
que precisa da internet para se reconstruir não fica bem.

## Testes e CI {#tests-and-ci}

O `.github/workflows/ci.yml` executa, em cada push e pull request:

1. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features`;
2. `cargo deny check licenses bans` — só licenças aprovadas pela OSI são
   permitidas pelo `deny.toml`; uma dependência copyleft ou de código
   disponível falha a build;
3. `npm run check` e `npm run build`;
4. a imagem (`linux/amd64`), apenas build.

O `.github/workflows/release.yml` publica a mesma imagem em
`ghcr.io/laupernoe/dumbmonit`: `:edge` em cada push para `main`, `:X.Y.Z`,
`:X.Y`, `:X` e `:latest` em cada tag `v*`.

Os testes de integração vivem em `crates/server/tests/`; os módulos puros
(conversão de métricas, análise de opções, máquina de estados de alertas,
silêncios) são testados no próprio local, sem rede.

## Adicionar um coletor {#adding-a-collector}

1. Implemente `Collector` de `crates/proto/src/collector.rs`: `kind()`,
   `probe(&Target) -> Result<Vec<Sample>, ProbeError>` e opcionalmente
   `discover()` para deteção de perfil. Devolva as variantes de `ProbeError` com
   verdade: `means_down()` decide se uma falha conta como «dispositivo em baixo»
   (alertada) ou como erro de configuração (mostrado no dispositivo, não
   notificado).
2. Leia as suas opções de `Target::tags` num `options.rs`, com predefinições.
3. Registe-o em `crates/server/src/main.rs` com `registry.register(...)`. O
   agendador e a API nunca conhecem tipos concretos.
4. Descreva-o em `crates/server/src/api/collectors.rs`: rótulo, resumo,
   exemplos, tipos de credencial, dica de endereço, aviso de configuração e a
   lista de `options` (chave, rótulo, ajuda, placeholder, predefinição, entrada,
   escolhas). A interface constrói o seu formulário a partir disto; um teste aí
   verifica que todo o tipo registado tem um aviso e que as chaves das opções
   correspondem ao que o coletor lê.
5. Nomeie as métricas `<what>_<unit>` sem o prefixo `dumbmonit_` (o escritor
   acrescenta-o) e acrescente qualquer métrica nova que uma regra incorporada
   vise à lista nos testes de `alerting/rules.rs`.
6. Adicione uma página em `docs/devices/`.

As amostras são `Sample::new(name, value, MetricKind::Gauge | Counter,
timestamp_ms)` com `.with_label(k, v)`; as etiquetas de identidade (`target`,
`host`, `tag_*`) são acrescentadas pelo registo. Não as emita você próprio:
nomeie uma etiqueta por objeto segundo o objeto (`proxy_host`, `esxi`,
`ceph_host`). Uma etiqueta em colisão é mantida como `exported_<name>`, e as
opções listadas para o tipo em `api/collectors.rs` não são copiadas como
`tag_*`.

## Adicionar um canal de notificação {#adding-a-notification-channel}

1. Implemente `Notifier` (`crates/server/src/notify/mod.rs`): `kind()` e
   `async fn send(&Message) -> Result<(), NotifyError>`. Os canais web partilham
   o cliente HTTP e os auxiliares em `notify/http.rs`; veja `services.rs`,
   `chat.rs`, `push.rs`, `oncall.rs` e `smtp.rs` para irmãos.
2. Adicione o tipo a `CHANNEL_KINDS` e ao `match` de `build` em
   `notify/mod.rs`, que converte um `ChannelConfig` guardado (definições +
   segredos decifrados) num `Notifier`.
3. Descreva-o em `notify/catalog.rs`: rótulo, resumo, âncora `doc_url`, e os
   campos `settings` e `secrets` (chave, rótulo, obrigatório, entrada, ajuda,
   placeholder, opções, forma, predefinição). A interface renderiza o formulário
   a partir desta descrição, pelo que não é preciso nenhum lançamento da
   interface.
4. Documente-o em `docs/notifications.md` em *Canais suportados*, com um título
   cujo slug corresponda à âncora `doc_url`: a interface liga diretamente a essa
   âncora no Read the Docs.

## Capturas de ecrã sem interface {#headless-screenshots}

O `web/tools/README.md` descreve como renderizar todas as páginas num Chromium
sem interface a correr no Docker (`zenika/alpine-chrome:with-puppeteer`, sem
precisar de navegador local): inicie sessão com `curl` para obter o cookie
`dumbmonit_session`, depois execute `web/tools/screenshot.js` contra o servidor
de pré-visualização ou diretamente contra `:8080`. Escreve
`web/shots/<page>-<theme>-<viewport>.png` para ambos os temas e ambas as
janelas de visualização, que é como a interface é verificada após uma alteração
e como foram feitas as capturas de ecrã desta documentação.

## Documentação {#documentation}

Este site é construído com MkDocs e Material for MkDocs a partir de `docs/`
(`mkdocs.yml` na raiz, `.readthedocs.yaml` para o Read the Docs). Para
pré-visualizar:

```bash
pip install -r docs/requirements.txt
mkdocs serve
mkdocs build --strict   # what Read the Docs runs; must pass with no warnings
```

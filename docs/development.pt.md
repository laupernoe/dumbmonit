# Desenvolvimento {#development}

O DumbMonit é um único binário Rust (`crates/server`, que incorpora o
build SvelteKit) mais o VictoriaMetrics para séries temporais, iniciado
pelo servidor a partir do binário incluído na imagem, e uma base de dados
SQLite incorporada para configuração e estado. Um contentor. O código e os
comentários Rust estão em francês; a interface web está em inglês.

## Disposição do projeto {#project-layout}

```
crates/proto     tipos partilhados: Sample, Target, Credential, trait Collector (+ ProbeError)
crates/server    o binário
  api/           rotas axum (mod.rs é a tabela de rotas); spa.rs serve web/build
  auth/          senha de instância única, cookie de sessão HttpOnly, limitação de taxa
  collectors/    snmp (profiles/*.yaml carregados no arranque), proxmox, pbs, synology, agent, uptime
  scheduler.rs   executa todo target ativado no seu intervalo através do registo de coletores
  tsdb/          escritor VictoriaMetrics (flush em lote) + proxy de consulta; embedded.rs supervisiona o VictoriaMetrics filho
  db/            SQLite + migrações numeradas em db/migrations/
  alerting/      regras, máquina de estados (Phase/EffectivePhase), supressão por pai, silêncios, linha de base sazonal
  notify/        22 canais de notificação, descritos à interface por notify/catalog.rs
  crypto.rs      AES-256-GCM para credenciais/tokens, chave derivada de /data/secret.key ou DUMBMONIT_SECRET
crates/agent     agente Linux/macOS/FreeBSD/Windows; install/ contém install.sh/.ps1 servidos pelo servidor
web/             SvelteKit (Svelte 5 runes, Tailwind 4, uPlot), adapter-static SPA fallback, ssr=false
profiles/        perfis de coleta SNMP, aplicados automaticamente pelo sysObjectID
docs/            esta documentação (MkDocs, publicada no Read the Docs)
```

## Compilar sem toolchain Rust {#building-without-a-rust-toolchain}

A compilação acontece no Docker; nada de Rust precisa de estar instalado
no anfitrião.

```bash
# Stack completa (constrói a imagem: cerca de 10 min a frio) — interface em http://localhost:8080
docker compose up -d --build
# camada de desenvolvimento: o VictoriaMetrics incorporado é publicado em :8428 para consultas MetricsQL diretas
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build

# Rust, dentro da fase builder (musl/alpine)
docker build -t dumbmonit-devenv --target builder .
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server --test alerts_api   # um ficheiro de teste de integração
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server test_name          # um teste pelo nome
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo clippy --all-targets --all-features -- -D warnings
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo fmt --all --check
```

Com o Rust instalado localmente, `cargo test` e `cargo clippy -- -D
warnings` funcionam diretamente. Edição 2024, MSRV 1.88; `rustfmt.toml`
define `max_width = 100` e `use_small_heuristics = "Max"`.

O build Docker é em camadas: build web → dependências Rust sobre
fontes fictícias → fontes reais (com um `touch` antes do segundo `cargo
build`, caso contrário o cargo reutiliza os artefactos fictícios) →
agente cross-compilado com cargo-zigbuild para `x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-musl` e `x86_64-pc-windows-gnu` → `FROM scratch`.
`web/build` tem de existir antes de `cargo build`, ou o servidor sai sem
interface.

## Interface web {#web-ui}

O Node 22 é suficiente no anfitrião.

```bash
cd web && npm run dev        # Vite em :5173, encaminha /api para localhost:8080 (o servidor Docker)
cd web && npm run check      # svelte-check (o CI executa isto)
cd web && npm run build      # build estático em web/build, incorporado pelo rust_embed no cargo build
```

Convenções: só Svelte 5 runes (`$props`, `$state`, `$derived`, `$effect`,
snippets, `onclick`). Todo acesso de rede passa por `src/lib/api/`; os
componentes nunca fazem `fetch`. Os tipos em `api/types.ts` espelham
exatamente as estruturas Rust e têm de ser mantidos sincronizados com
`crates/server/src/api/*.rs`. O sistema de design (tokens em
`src/app.css`, primitivas em `src/lib/ui/`) está documentado em
`DESIGN.md`; o contexto do produto em `PRODUCT.md`. Nunca execute dois
`vite build` ao mesmo tempo: limpam `.svelte-kit/output`.

## Traduções (Weblate) {#translations-weblate}

A interface web está preparada para tradução comunitária mas nada está
ainda traduzido: o inglês é o único locale distribuído, em
`web/messages/en.json` (plano, chaves com pontos, marcadores `{param}`,
compilado pelo [Paraglide JS](https://paraglidejs.com) em funções de
mensagem com tree-shaking sob `web/src/lib/paraglide/`, gerado e não
versionado). A tradução acontece em
[hosted.weblate.org/projects/dumbmonit](https://hosted.weblate.org/projects/dumbmonit/)
(componente `web-ui`), configurado como se segue:

| Definição | Valor |
| --- | --- |
| Repositório | este repositório, subdiretório `web/` |
| Máscara de ficheiro | `web/messages/*.json` |
| Ficheiro de idioma base monolingue | `web/messages/en.json` |
| Formato de ficheiro | Ficheiro JSON (monolingue) |
| Novo idioma | criar um novo ficheiro de tradução (o Weblate escreve `web/messages/<locale>.json`) |
| Controlo de versões | GitHub, através da app Weblate GitHub: as traduções voltam como pull requests em `main` |

Novas strings da interface entram apenas em `web/messages/en.json`; um
tradutor depois preenche os outros ficheiros de locale através do
Weblate. As chaves são extraídas progressivamente — hoje isso cobre as
páginas de Settings (`web/src/routes/settings/`) e o texto incorporado de
algumas primitivas de `web/src/lib/ui/` — o resto da interface ainda lê
inglês simples e é extraído conforme é tocado. Ver `CONTRIBUTING.md` para
a convenção de escrita de strings.

`web/project.inlang/settings.json` aponta o seu único plugin
(message-format) para `./node_modules/@inlang/plugin-message-format/dist/index.js`
em vez do URL jsdelivr para onde as próprias ferramentas do inlang
apontam por predefinição: um caminho relativo, não-`http`, é lido
diretamente do disco, por isso `npm run build` nunca contacta uma CDN —
importante já que o build da imagem não tem garantia de uma cache npm
quente a oferecer esse URL, e uma ferramenta de homelab que precisa da
internet para se reconstruir a si própria é mal visto.

## Testes e CI {#tests-and-ci}

`.github/workflows/ci.yml` executa, em cada push e pull request:

1. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features`;
2. `cargo deny check licenses bans` — só licenças aprovadas pela OSI são
   permitidas por `deny.toml`; uma dependência copyleft ou source-available
   faz o build falhar;
3. `npm run check` e `npm run build`;
4. a imagem (`linux/amd64`), só build.

`.github/workflows/release.yml` envia a mesma imagem para
`ghcr.io/laupernoe/dumbmonit`: `:edge` em cada push para `main`, `:X.Y.Z`,
`:X.Y`, `:X` e `:latest` em cada tag `v*`.

Os testes de integração vivem em `crates/server/tests/`; os módulos puros
(conversão de métricas, análise de opções, máquina de estados de alerta,
silêncios) são testados no local sem rede.

## Adicionar um coletor {#adding-a-collector}

1. Implemente `Collector` a partir de `crates/proto/src/collector.rs`:
   `kind()`, `probe(&Target) -> Result<Vec<Sample>, ProbeError>` e
   opcionalmente `discover()` para deteção de perfil. Devolva as variantes
   de `ProbeError` com sinceridade: `means_down()` decide se uma falha
   conta como «dispositivo em baixo» (alertado) ou como um erro de
   configuração (mostrado no dispositivo, não notificado).
2. Leia as suas opções de `Target::tags` num `options.rs`, com
   predefinições.
3. Registe-o em `crates/server/src/main.rs` com `registry.register(...)`.
   O escalonador e a API nunca conhecem tipos concretos.
4. Descreva-o em `crates/server/src/api/collectors.rs`: rótulo, resumo,
   exemplos, tipos de credencial, sugestão de endereço, aviso de
   configuração e a lista de `options` (chave, rótulo, ajuda, marcador de
   posição, predefinição, entrada, escolhas). A interface constrói o seu
   formulário a partir disto; um teste aí verifica que todo tipo
   registado tem um aviso e que as chaves de opção correspondem ao que o
   coletor lê.
5. Nomeie as métricas `<o-quê>_<unidade>` sem o prefixo `dumbmonit_` (o
   escritor adiciona-o) e adicione qualquer métrica nova que uma regra
   incorporada visa à lista em `alerting/rules.rs` testes.
6. Adicione uma página sob `docs/devices/`.

As amostras são `Sample::new(name, value, MetricKind::Gauge | Counter,
timestamp_ms)` com `.with_label(k, v)`; as etiquetas de identidade
(`target`, `host`, `tag_*`) são adicionadas pelo registo.

## Adicionar um canal de notificação {#adding-a-notification-channel}

1. Implemente `Notifier` (`crates/server/src/notify/mod.rs`): `kind()` e
   `async fn send(&Message) -> Result<(), NotifyError>`. Os canais web
   partilham o cliente HTTP e os auxiliares em `notify/http.rs`; veja
   `services.rs`, `chat.rs`, `push.rs`, `oncall.rs` e `smtp.rs` para
   exemplos semelhantes.
2. Adicione o tipo a `CHANNEL_KINDS` e ao match `build` em `notify/mod.rs`,
   que transforma um `ChannelConfig` guardado (definições + segredos
   desencriptados) num `Notifier`.
3. Descreva-o em `notify/catalog.rs`: rótulo, resumo, âncora `doc_url`, e
   os campos `settings` e `secrets` (chave, rótulo, obrigatório, entrada,
   ajuda, marcador de posição, opções, forma, predefinição). A interface
   desenha o formulário a partir desta descrição, por isso nenhuma versão
   da interface é necessária.
4. Documente-o em `docs/notifications.md` sob *Supported channels*, com um
   título cujo slug corresponda à âncora `doc_url`: a interface liga
   diretamente a essa âncora no Read the Docs.

## Capturas de ecrã automáticas {#headless-screenshots}

`web/tools/README.md` descreve como renderizar cada página num Chromium
headless a correr no Docker (`zenika/alpine-chrome:with-puppeteer`, sem
necessidade de navegador local): inicie sessão com `curl` para obter o
cookie `dumbmonit_session`, depois execute `web/tools/screenshot.js`
contra o servidor de pré-visualização ou diretamente contra `:8080`.
Escreve `web/shots/<página>-<tema>-<viewport>.png` para ambos os temas e
ambos os viewports, que é como a interface é verificada depois de uma
alteração e como as capturas de ecrã desta documentação foram feitas.

## Documentação {#documentation}

Este site é construído com MkDocs e Material for MkDocs a partir de
`docs/` (`mkdocs.yml` na raiz, `.readthedocs.yaml` para o Read the Docs).
Para pré-visualizar:

```bash
pip install -r docs/requirements.txt
mkdocs serve
mkdocs build --strict   # o que o Read the Docs executa; tem de passar sem avisos
```

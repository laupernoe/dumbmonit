# Desenvolvimento {#development}

O DumbMonit é um único binário Rust (`crates/server`, que embute o build
SvelteKit) mais o VictoriaMetrics para séries temporais, iniciado pelo
servidor a partir do binário entregue na imagem, e um banco de dados SQLite
embutido para configuração e estado. Um contêiner. Código e comentários em
Rust estão em francês; a interface web está em inglês.

## Estrutura do projeto {#project-layout}

```
crates/proto     tipos compartilhados: Sample, Target, Credential, trait Collector (+ ProbeError)
crates/server    o binário
  api/           rotas axum (mod.rs é a tabela de rotas); spa.rs serve web/build
  auth/          senha de instância única, cookie de sessão HttpOnly, limite de taxa
  collectors/    snmp (profiles/*.yaml carregados na inicialização), proxmox, pbs, synology, agent, uptime
  scheduler.rs   executa todo target ativado em seu intervalo através do registro de coletores
  tsdb/          escritor VictoriaMetrics (flush em lote) + proxy de consulta; embedded.rs supervisiona o VictoriaMetrics filho
  db/            SQLite + migrações numeradas em db/migrations/
  alerting/      regras, máquina de estados (Phase/EffectivePhase), supressão pelo pai, silêncios, linha de base sazonal
  notify/        22 canais de notificação, descritos para a interface por notify/catalog.rs
  crypto.rs      AES-256-GCM para credenciais/tokens, chave derivada de /data/secret.key ou DUMBMONIT_SECRET
crates/agent     agente Linux/macOS/FreeBSD/Windows; install/ contém install.sh/.ps1 servidos pelo servidor
web/             SvelteKit (Svelte 5 runes, Tailwind 4, uPlot), fallback SPA adapter-static, ssr=false
profiles/        perfis de coleta SNMP, aplicados automaticamente por sysObjectID
docs/            esta documentação (MkDocs, publicada no Read the Docs)
```

## Compilando sem um toolchain Rust {#building-without-a-rust-toolchain}

A compilação acontece no Docker; nada de Rust precisa ser instalado no
host.

```bash
# Stack completa (constrói a imagem: cerca de 10 min a frio) — interface em http://localhost:8080
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build

# Rust, dentro do estágio builder (musl/alpine)
docker build -t dumbmonit-devenv --target builder .
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server --test alerts_api   # um arquivo de teste de integração
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server test_name          # um teste pelo nome
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo clippy --all-targets --all-features -- -D warnings
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo fmt --all --check
```

Com o Rust instalado localmente, `cargo test` e
`cargo clippy -- -D warnings` funcionam diretamente. Edição 2024, MSRV
1.88; `rustfmt.toml` define `max_width = 100` e
`use_small_heuristics = "Max"`.

O build Docker é em camadas: build web → dependências Rust em fontes fictícias
→ fontes reais (com um `touch` antes do segundo `cargo build`, senão o
cargo reutiliza os artefatos fictícios) → agente cross-compilado com
cargo-zigbuild para `x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-musl` e `x86_64-pc-windows-gnu` → `FROM scratch`.
`web/build` precisa existir antes de `cargo build`, senão o servidor é
enviado sem interface.

## Interface web {#web-ui}

Node 22 é suficiente no host.

```bash
cd web && npm run dev        # Vite em :5173, redireciona /api para localhost:8080 (o servidor Docker)
cd web && npm run check      # svelte-check (o CI executa isso)
cd web && npm run build      # build estático em web/build, embutido por rust_embed no cargo build
```

Convenções: apenas Svelte 5 runes (`$props`, `$state`, `$derived`,
`$effect`, snippets, `onclick`). Todo acesso à rede passa por
`src/lib/api/`; componentes nunca fazem `fetch`. Tipos em `api/types.ts`
espelham exatamente as structs Rust e precisam ser mantidos em sincronia
com `crates/server/src/api/*.rs`. O design system (tokens em
`src/app.css`, primitivas em `src/lib/ui/`) é documentado em `DESIGN.md`; o
contexto de produto em `PRODUCT.md`. Nunca execute dois `vite build`s ao
mesmo tempo: eles destroem `.svelte-kit/output`.

## Traduções {#translations}

A interface web é traduzida por pull request, como qualquer outra contribuição. O idioma de origem é o inglês (`web/messages/en.json`: chaves planas, marcadores `{param}`); cada outro idioma é um arquivo, `web/messages/<locale>.json`, compilado pelo [Paraglide JS](https://paraglidejs.com) em funções de mensagem com tree-shaking em `web/src/lib/paraglide/` (gerado, não versionado).

1. Edite `web/messages/<locale>.json` (`fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans`), ou copie `en.json` para adicionar um novo idioma.
2. Mantenha todas as chaves e todas as `{variables}` de `en.json`; traduza apenas os valores. Sem marcação HTML nas mensagens.
3. Os plurais usam a sintaxe de mensagens, não strings montadas à mão: mantenha a estrutura `match`/`plural` da mensagem em inglês e traduza cada ramo.
4. Execute `cd web && npm run i18n:check`: falha se um idioma não tiver uma chave ou mudar suas `{variables}`.
5. Abra um pull request.

As novas strings da interface são adicionadas primeiro a `web/messages/en.json` (o único arquivo de que um pull request de funcionalidade precisa; `npm run i18n:merge` junta fragmentos temporários). Um idioma aparece no seletor (Configurações, Aparência) assim que seu arquivo existe. A convenção de nomes de chaves está em `CONTRIBUTING.md`.

`web/project.inlang/settings.json` aponta seu único plugin
(message-format) para `./node_modules/@inlang/plugin-message-format/dist/index.js`
em vez da URL jsdelivr que a própria ferramenta do inlang usa por padrão:
um caminho relativo, não-`http`, é lido direto do disco, então
`npm run build` nunca alcança um CDN — importante já que o build da imagem
não tem garantia de um cache npm aquecido oferecendo essa URL, e uma
ferramenta de homelab que precisa da internet para se reconstruir é feia.

## Testes e CI {#tests-and-ci}

`.github/workflows/ci.yml` executa, em todo push e pull request:

1. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features --
   -D warnings`, `cargo test --all-features`;
2. `cargo deny check licenses bans` — apenas licenças aprovadas pela OSI
   são permitidas por `deny.toml`; uma dependência copyleft ou
   source-available falha o build;
3. `npm run check` e `npm run build`;
4. a imagem (`linux/amd64`), apenas build.

`.github/workflows/release.yml` envia a mesma imagem para
`ghcr.io/laupernoe/dumbmonit`: `:edge` em todo push para `main`, `:X.Y.Z`,
`:X.Y`, `:X` e `:latest` em toda tag `v*`.

Testes de integração vivem em `crates/server/tests/`; módulos puros
(conversão de métricas, análise de opções, máquina de estados de alerta,
silêncios) são testados no lugar, sem rede.

## Adicionando um coletor {#adding-a-collector}

1. Implemente `Collector` de `crates/proto/src/collector.rs`: `kind()`,
   `probe(&Target) -> Result<Vec<Sample>, ProbeError>` e opcionalmente
   `discover()` para detecção de perfil. Retorne variantes `ProbeError`
   com veracidade: `means_down()` decide se uma falha conta como
   "monitor fora do ar" (alertado) ou como um erro de configuração
   (mostrado no monitor, não notificado).
2. Leia suas opções de `Target::tags` em um `options.rs`, com padrões.
3. Registre-o em `crates/server/src/main.rs` com `registry.register(...)`.
   O agendador e a API nunca conhecem tipos concretos.
4. Descreva-o em `crates/server/src/api/collectors.rs`: label, resumo,
   exemplos, tipos de credencial, dica de endereço, aviso de configuração
   e a lista `options` (chave, label, ajuda, placeholder, padrão, input,
   escolhas). A interface constrói seu formulário a partir disso; um
   teste ali verifica que todo tipo registrado tem um aviso e que as
   chaves de opção correspondem ao que o coletor lê.
5. Nomeie métricas `<o-que>_<unidade>` sem o prefixo `dumbmonit_` (o
   escritor o adiciona) e adicione qualquer métrica nova que uma regra
   integrada tem como alvo à lista nos testes de `alerting/rules.rs`.
6. Adicione uma página sob `docs/devices/`.

Amostras são `Sample::new(name, value, MetricKind::Gauge | Counter,
timestamp_ms)` com `.with_label(k, v)`; rótulos de identidade (`target`,
`host`, `tag_*`) são adicionados pelo registro.

## Adicionando um canal de notificação {#adding-a-notification-channel}

1. Implemente `Notifier` (`crates/server/src/notify/mod.rs`): `kind()` e
   `async fn send(&Message) -> Result<(), NotifyError>`. Canais web
   compartilham o cliente HTTP e helpers em `notify/http.rs`; veja
   `services.rs`, `chat.rs`, `push.rs`, `oncall.rs` e `smtp.rs` para
   exemplos semelhantes.
2. Adicione o tipo a `CHANNEL_KINDS` e ao match `build` em
   `notify/mod.rs`, que transforma um `ChannelConfig` armazenado
   (configurações + segredos descriptografados) em um `Notifier`.
3. Descreva-o em `notify/catalog.rs`: label, resumo, âncora `doc_url`, e
   os campos `settings` e `secrets` (chave, label, obrigatório, input,
   ajuda, placeholder, opções, forma, padrão). A interface renderiza o
   formulário a partir dessa descrição, então nenhuma versão de interface
   é necessária.
4. Documente-o em `docs/notifications.md` sob *Canais suportados*, com um
   título cujo slug corresponda à âncora `doc_url`: a interface liga
   diretamente para essa âncora no Read the Docs.

## Capturas de tela headless {#headless-screenshots}

`web/tools/README.md` descreve como renderizar toda página em um Chromium
headless rodando no Docker (`zenika/alpine-chrome:with-puppeteer`, sem
necessidade de navegador local): faça login com `curl` para obter o
cookie `dumbmonit_session`, depois execute `web/tools/screenshot.js`
contra o servidor de preview ou diretamente contra `:8080`. Ele escreve
`web/shots/<page>-<theme>-<viewport>.png` para ambos os temas e ambos os
viewports, que é como a interface é verificada após uma mudança e como as
capturas de tela desta documentação foram feitas.

## Documentação {#documentation}

Este site é construído com MkDocs e Material for MkDocs a partir de
`docs/` (`mkdocs.yml` na raiz, `.readthedocs.yaml` para o Read the Docs).
Para pré-visualizar:

```bash
pip install -r docs/requirements.txt
mkdocs serve
mkdocs build --strict   # o que o Read the Docs executa; precisa passar sem avisos
```

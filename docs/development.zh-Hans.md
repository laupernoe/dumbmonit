# 开发 {#development}

DumbMonit 是一个 Rust 二进制文件(`crates/server`,其嵌入 SvelteKit
构建)加上用于时间序列的 VictoriaMetrics,由服务器从
二进制文件启动的镜像,以及用于配置和状态的嵌入式 SQLite 数据库。一个容器。Rust 代码和注释使用法语;网络 UI
使用英语。

## 项目布局 {#project-layout}

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

## 没有 Rust 工具链编译 {#building-without-a-rust-toolchain}

编译发生在 Docker 中;主机上不需要安装任何 Rust。

```bash
# 完整堆栈(构建镜像:大约 10 分钟冷启动) — UI on http://localhost:8080
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build

# Rust,在 builder 阶段内(musl/alpine)
docker build -t dumbmonit-devenv --target builder .
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server --test alerts_api   # one integration test file
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server test_name          # one test by name
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo clippy --all-targets --all-features -- -D warnings
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo fmt --all --check
```

安装了 Rust 本地,`cargo test`和`cargo clippy -- -D warnings`
直接工作。Edition 2024,MSRV 1.95;`rustfmt.toml`设置`max_width = 100`
和`use_small_heuristics = "Max"`。

Docker 构建分层:web 构建 → 假源上的 Rust 依赖 →
真实源(在第二个`cargo build`前用`touch`,否则 cargo
重用假工件) → 代理与 cargo-zigbuild 交叉编译为
`x86_64-unknown-linux-musl`、`aarch64-unknown-linux-musl`和
`x86_64-pc-windows-gnu` → `FROM scratch`。`web/build`必须在之前存在
`cargo build`,否则服务器没有 UI。

## 网络 UI {#web-ui}

主机上 Node 22 足够。

```bash
cd web && npm run dev        # Vite on :5173, proxies /api to localhost:8080 (the Docker server)
cd web && npm run check      # svelte-check (CI runs this)
cd web && npm run build      # static build into web/build, embedded by rust_embed at cargo build
```

约定:Svelte 5 仅有绳索(`$props`、`$state`、`$derived`、`$effect`,
片段,`onclick`)。所有网络访问通过`src/lib/api/`;
组件永不`fetch`。类型在`api/types.ts`精确镜像 Rust 结构
并必须与`crates/server/src/api/*.rs`保持同步。设计
系统生活在`src/app.css`(代币)和`src/lib/ui/`(原始)。永不运行两个`vite build`
同时:它们擦拭`.svelte-kit/output`。

## 翻译 {#translations}

Web 界面与其他贡献一样，通过 pull request 翻译。源语言为英语（`web/messages/en.json`：扁平键、`{param}` 占位符）；其他每种语言对应一个文件 `web/messages/<locale>.json`，由 [Paraglide JS](https://paraglidejs.com) 编译为支持 tree-shaking 的消息函数，位于 `web/src/lib/paraglide/`（自动生成，不纳入版本控制）。

1. 编辑 `web/messages/<locale>.json`（`fr`、`de`、`es`、`it`、`pt`、`pt-BR`、`ru`、`zh-Hans`），或复制 `en.json` 以添加新语言。
2. 保留 `en.json` 中的所有键和所有 `{variables}`；只翻译值。消息中不要使用 HTML 标记。
3. 复数使用消息语法，而不是手工拼接的字符串：保留英文消息的 `match`/`plural` 结构，并翻译每个分支。
4. 运行 `cd web && npm run i18n:check`：如果某种语言缺少键或改动了 `{variables}`，检查会失败。
5. 提交 pull request。

新的界面文本先添加到 `web/messages/en.json`（功能 pull request 只需要这个文件；`npm run i18n:merge` 会合并临时片段）。语言文件一存在，该语言就会出现在语言选择器中（设置，外观）。键的命名约定见 `CONTRIBUTING.md`。

`web/project.inlang/settings.json`指向其一个插件(message-format)在
`./node_modules/@inlang/plugin-message-format/dist/index.js`而不是
jsdelivr URL inlang 自己的工具默认:相对的,非`http`路径是
直接从磁盘读取,所以`npm run build`永不达到 CDN —
重要因为镜像构建没有保证的温暖 npm 缓存提供
那 URL,以及 homelab 工具需要互联网自重新构建的是
坏看起来。

## 测试和 CI {#tests-and-ci}

`.github/workflows/ci.yml`运行,在每个推送和拉取请求:

1. `cargo fmt --all --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-features`;
2. `cargo deny check licenses bans` — 仅 OSI 批准的许可证允许
   由`deny.toml`;copyleft 或源可用依赖失败构建;
3. `npm run check`和`npm run build`;
4. 镜像(`linux/amd64`),仅构建。

`.github/workflows/release.yml`推送相同镜像到
`ghcr.io/laupernoe/dumbmonit`:`:edge`在每个推送到`main`,`:X.Y.Z`,
`:X.Y`、`:X`和`:latest`在每个`v*`标签。

集成测试生活在`crates/server/tests/`;纯模块(度量
转换、选项解析、告警状态机、沉默)被测试
不网络的地方。

## 添加收集器 {#adding-a-collector}

1. 从`crates/proto/src/collector.rs`实现`Collector`:
   `kind()`、`probe(&Target) -> Result<Vec<Sample>, ProbeError>`和可选
   `discover()`用于配置文件检测。返回`ProbeError`变体
   真实:` means_down()`决定失败是否计为"监控项
   关闭"(告警)或作为配置错误(显示在监控项,不
   通知)。
2. 从`Target::tags`在`options.rs`中读取你的选项,带默认。
3. 在`crates/server/src/main.rs`中用`registry.register(...)`注册它。
   调度器和 API 永不知道具体类型。
4. 在`crates/server/src/api/collectors.rs`中描述它:标签、摘要、
   示例、凭证类型、地址提示、设置通知和`options`
   列表(键、标签、帮助、占位符、默认、输入、选择)。UI 构建
   其形式从此;一个测试那里检查每个已注册的类型有通知
   以及选项键匹配收集器读取。
5. 命名度量`<what>_<unit>`不用`dumbmonit_`前缀(写入
   添加)并添加任何新度量内置规则针对列表在
   `alerting/rules.rs`测试。
6. 在`docs/devices/`下添加一个页面。

样本是`Sample::new(name, value, MetricKind::Gauge | Counter,
timestamp_ms)`带`.with_label(k, v)`;身份标签(`target`、`host`、
`tag_*`)由注册表添加。不自己发出它们:命名一个按对象的标签
对象(`proxy_host`、`esxi`、`ceph_host`)。碰撞标签
保持为`exported_<name>`,以及为类型列出的选项在
`api/collectors.rs`不复制为`tag_*`。

## 添加通知通道 {#adding-a-notification-channel}

1. 实现`Notifier`(`crates/server/src/notify/mod.rs`):
   `kind()`和`async fn send(&Message) -> Result<(), NotifyError>`。网络通道共享
   HTTP 客户端和`notify/http.rs`中的帮助者;查看`services.rs`、
   `chat.rs`、`push.rs`、`oncall.rs`和`smtp.rs`用于兄弟。
2. 添加类型到`CHANNEL_KINDS`和到`notify/mod.rs`中的`build`匹配,
   将存储的`ChannelConfig`(设置 + 解密
   秘密)变成`Notifier`。
3. 在`notify/catalog.rs`中描述它:标签、摘要、`doc_url`锚点,以及
   `settings`和`secrets`字段(键、标签、必要、输入、帮助、
   占位符、选项、形状、默认)。UI 从此
   描述呈现形式,所以不需要 UI 发布。
4. 在`docs/notifications.md`中在*支持的通道*下记录它,带有
   标题其 slug 匹配`doc_url`锚点:UI 链接直接到
   那个锚点在 Read the Docs 上。

## 无头截图 {#headless-screenshots}

`web/tools/README.md`描述如何呈现每个页面
在 Docker 中运行的无头 Chromium(`zenika/alpine-chrome:with-puppeteer`,
无需本地浏览器):用`curl`登录以获取`dumbmonit_session`
cookie,然后针对预览服务器运行`web/tools/screenshot.js`或
直接对`:8080`。它写`web/shots/<page>-<theme>-<viewport>.png`
用于两个主题和两个视口,这是如何 UI 被检查后
改变以及如何这个文档的截图被制作。

## 文档 {#documentation}

此站点由 MkDocs 和 Material for MkDocs 从`docs/`构建
(`mkdocs.yml`在根,`.readthedocs.yaml`用于 Read the Docs)。预览:

```bash
pip install -r docs/requirements.txt
mkdocs serve
mkdocs build --strict   # what Read the Docs runs; must pass with no warnings
```

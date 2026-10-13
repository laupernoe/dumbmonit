# Разработка

DumbMonit — один бинарный файл Rust (`crates/server`, который встраивает сборку SvelteKit) плюс VictoriaMetrics для временных рядов, запущенный сервером из двоичного файла, отправленного в образе, и встроенная база данных SQLite для конфигурации и состояния. Один контейнер. Код и комментарии на Rust на французском; веб-интерфейс на английском.

## Макет проекта

```
crates/proto     общие типы: Sample, Target, Credential, trait Collector (+ ProbeError)
crates/server    бинарный файл
  api/           маршруты axum (mod.rs — таблица маршрутов); spa.rs обслуживает web/build
  auth/          пароль одного экземпляра, HttpOnly сеансовый cookie, ограничение скорости
  collectors/    snmp (профили/*.yaml загружаются при запуске), proxmox, pbs, synology, agent, uptime
  scheduler.rs   запускает каждое включенное целевое устройство по его интервалу через реестр сборщика
  tsdb/          писатель VictoriaMetrics (пакетная очистка) + прокси запроса; embedded.rs контролирует дочерний VictoriaMetrics
  db/            SQLite + нумерованные миграции в db/migrations/
  alerting/      правила, конечный автомат (Phase/EffectivePhase), подавление родителем, молчание, сезонная базовая линия
  notify/        22 канала уведомлений, описанные интерфейсом пользователя через notify/catalog.rs
  crypto.rs      AES-256-GCM для credentials/tokens, ключ получен из /data/secret.key или DUMBMONIT_SECRET
crates/agent     агент Linux/macOS/FreeBSD/Windows; install/ содержит install.sh/.ps1, обслуживаемые сервером
web/             SvelteKit (Svelte 5 runes, Tailwind 4, uPlot), adapter-static SPA fallback, ssr=false
profiles/        профили сбора SNMP, автоматически применяются по sysObjectID
docs/            эта документация (MkDocs, опубликована на Read the Docs)
```

## Построение без локального набора инструментов Rust

Компиляция происходит в Docker; на хосте не требуется устанавливать ничего, что нужно для Rust.

```bash
# Полный стек (создает образ: около 10 мин в холодном состоянии) — интерфейс на http://localhost:8080
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build

# Rust, внутри этапа builder (musl/alpine)
docker build -t dumbmonit-devenv --target builder .
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server --test alerts_api   # один файл интеграционного теста
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server test_name          # один тест по имени
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo clippy --all-targets --all-features -- -D warnings
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo fmt --all --check
```

С установленным локально Rust, `cargo test` и `cargo clippy -- -D warnings` работают напрямую. Edition 2024, MSRV 1.88; `rustfmt.toml` устанавливает `max_width = 100` и `use_small_heuristics = "Max"`.

Сборка Docker слоистая: сборка веб → зависимости Rust на dummy sources → реальные sources (с `touch` перед второй `cargo build`, иначе cargo повторно использует артефакты dummy) → агент кросс-компилируется с cargo-zigbuild для `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` и `x86_64-pc-windows-gnu` → `FROM scratch`. `web/build` должен существовать перед `cargo build`, или сервер отправляется без интерфейса.

## Веб-интерфейс

Node 22 достаточно на хосте.

```bash
cd web && npm run dev        # Vite на :5173, проксирует /api на localhost:8080 (сервер Docker)
cd web && npm run check      # svelte-check (CI запускает это)
cd web && npm run build      # статическая сборка в web/build, встроена rust_embed при cargo build
```

Соглашения: только Svelte 5 runes (`$props`, `$state`, `$derived`, `$effect`, snippets, `onclick`). Весь сетевой доступ проходит через `src/lib/api/`; компоненты никогда не `fetch`. Типы в `api/types.ts` точно отражают структуры Rust — их необходимо синхронизировать с `crates/server/src/api/*.rs`. Система дизайна находится в `src/app.css` (tokens) и `src/lib/ui/` (primitives). Никогда не запускайте два `vite build` одновременно: они удаляют `.svelte-kit/output`.

## Переводы (Weblate)

Веб-интерфейс готов к переводу сообщества, но ничего еще не переведено: английский — единственная поставляемая локаль, в `web/messages/en.json` (плоский, dotted keys, `{param}` placeholders, скомпилировано [Paraglide JS](https://paraglidejs.com) в функции сообщений, готовые к tree-shaking, под `web/src/lib/paraglide/`, сгенерировано и не отслеживается). Перевод происходит на [hosted.weblate.org/projects/dumbmonit](https://hosted.weblate.org/projects/dumbmonit/) (компонент `web-ui`), настроено следующим образом:

| Параметр | Значение |
| --- | --- |
| Репозиторий | этот репо, подпапка `web/` |
| Маска файла | `web/messages/*.json` |
| Монолингвальный базовый языковой файл | `web/messages/en.json` |
| Формат файла | JSON файл (монолингвальный) |
| Новый язык | создать новый файл перевода (Weblate пишет `web/messages/<locale>.json`) |
| Контроль версий | GitHub, через приложение Weblate GitHub: переводы возвращаются как pull requests на `main` |

Новые строки интерфейса появляются только в `web/messages/en.json`; переводчик затем заполняет другие файлы локали через Weblate. Ключи извлекаются постепенно — сегодня это охватывает страницы Параметров (`web/src/routes/settings/`) и встроенную копию пары примитивов `web/src/lib/ui/` — остальной интерфейс все еще читает простой английский и извлекается при прикосновении. См. `CONTRIBUTING.md` для соглашения об авторстве строк.

`web/project.inlang/settings.json` указывает свой единственный плагин (message-format) на `./node_modules/@inlang/plugin-message-format/dist/index.js` вместо URL jsdelivr, который по умолчанию использует собственный инструментарий inlang: относительный путь без `http` читается прямо с диска, поэтому `npm run build` никогда не достигает CDN — важно, так как сборка образа не гарантирует теплый npm кэш, предлагающий этот URL, и инструмент homelab, которому нужен интернет для перестройки себя, — это плохо.

## Тесты и CI

`.github/workflows/ci.yml` запускает при каждом push и pull request:

1. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features`;
2. `cargo deny check licenses bans` — в `deny.toml` разрешены только лицензии, одобренные OSI; copyleft или лицензия с открытым исходным кодом зависимость не проходит сборку;
3. `npm run check` и `npm run build`;
4. образ (`linux/amd64`), только сборка.

`.github/workflows/release.yml` отправляет тот же образ на `ghcr.io/laupernoe/dumbmonit`: `:edge` при каждом push на `main`, `:X.Y.Z`, `:X.Y`, `:X` и `:latest` при каждом теге `v*`.

Интеграционные тесты находятся в `crates/server/tests/`; чистые модули (преобразование метрик, разбор опций, конечный автомат состояния оповещения, молчание) тестируются на месте без сети.

## Добавление сборщика

1. Реализуйте `Collector` из `crates/proto/src/collector.rs`: `kind()`, `probe(&Target) -> Result<Vec<Sample>, ProbeError>` и опционально `discover()` для обнаружения профиля. Верните варианты `ProbeError` правдиво: `means_down()` решает, подсчитывается ли отказ как "устройство не работает" (оповещение) или как ошибка конфигурации (показано на устройстве, не уведомляется).
2. Прочитайте ваши опции из `Target::tags` в `options.rs` с значениями по умолчанию.
3. Зарегистрируйте его в `crates/server/src/main.rs` с `registry.register(...)`. Планировщик и API никогда не знают конкретные типы.
4. Описите его в `crates/server/src/api/collectors.rs`: метка, резюме, примеры, типы учетных данных, подсказка адреса, примечание установки и список `options` (ключ, метка, справка, заполнитель, по умолчанию, ввод, выбор). Интерфейс строит свою форму на основе этого; тест там проверяет, что каждый зарегистрированный вид имеет примечание и что ключи опций совпадают с тем, что читает сборщик.
5. Назовите метрики `<what>_<unit>` без префикса `dumbmonit_` (писатель добавляет его) и добавьте любую новую метрику, на которую нацелено встроенное правило, в список в тестах `alerting/rules.rs`.
6. Добавьте страницу в `docs/devices/`.

Примеры — `Sample::new(name, value, MetricKind::Gauge | Counter, timestamp_ms)` с `.with_label(k, v)`; метки идентификации (`target`, `host`, `tag_*`) добавляются реестром. Не излучайте их самостоятельно: назовите метку для каждого объекта по объекту (`proxy_host`, `esxi`, `ceph_host`). Столкновение метки сохраняется как `exported_<name>`, и опции, перечисленные для типа в `api/collectors.rs`, не копируются как `tag_*`.

## Добавление канала уведомлений

1. Реализуйте `Notifier` (`crates/server/src/notify/mod.rs`): `kind()` и `async fn send(&Message) -> Result<(), NotifyError>`. Веб-каналы совместно используют HTTP-клиент и помощников в `notify/http.rs`; посмотрите на `services.rs`, `chat.rs`, `push.rs`, `oncall.rs` и `smtp.rs` для товарищей.
2. Добавьте тип в `CHANNEL_KINDS` и в матч `build` в `notify/mod.rs`, который превращает сохраненную `ChannelConfig` (параметры + расшифрованные секреты) в `Notifier`.
3. Опишите его в `notify/catalog.rs`: метка, резюме, якорь `doc_url` и поля `settings` и `secrets` (ключ, метка, требуется, ввод, справка, заполнитель, опции, форма, по умолчанию). Интерфейс визуализирует форму на основе этого описания, поэтому выпуск пользовательского интерфейса не требуется.
4. Документируйте его в `docs/notifications.md` под *Supported channels*, с заголовком, слаг которого совпадает с якорем `doc_url`: интерфейс ссылается прямо на этот якорь на Read the Docs.

## Снимки экрана без головы

`web/tools/README.md` описывает, как визуализировать каждую страницу в бесголовном Chromium, работающем в Docker (`zenika/alpine-chrome:with-puppeteer`, локальный браузер не требуется): войдите с помощью `curl` для получения cookie `dumbmonit_session`, затем запустите `web/tools/screenshot.js` на сервере предварительного просмотра или прямо на `:8080`. Он записывает `web/shots/<page>-<theme>-<viewport>.png` для обеих тем и обоих представлений, что является тем, как интерфейс проверяется после изменения и как были созданы снимки экрана этой документации.

## Документация

Этот сайт построен с помощью MkDocs и Material for MkDocs из `docs/` (`mkdocs.yml` в корне, `.readthedocs.yaml` для Read the Docs). Для предварительного просмотра:

```bash
pip install -r docs/requirements.txt
mkdocs serve
mkdocs build --strict   # то, что запускает Read the Docs; должен пройти без предупреждений
```

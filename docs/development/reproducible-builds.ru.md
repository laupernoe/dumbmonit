# Воспроизводимые сборки

Сборка одного и того же коммита дважды даёт одни и те же байты. Две независимые
сборки коммита (чистые сборщики, без кэша, в разных каталогах, с разными датами
файлов) дают побитово идентичные:

- бинарный файл сервера (`dumbmonit`, статический musl, с встроенным веб-интерфейсом);
- бинарные файлы агента для Linux x86_64 и aarch64, Windows x86_64 и FreeBSD
  x86_64;
- файлы веб-интерфейса, встроенные в сервер;
- образ сервера и образ агента, сравниваемые по дайджесту манифеста OCI
  (каждый слой и конфигурация образа), при сборке с датой коммита в
  качестве `SOURCE_DATE_EPOCH`;
- бинарные файлы агента для macOS, при сборке на одной и той же версии macOS и Xcode.

Это проверяется для каждого pull request, затрагивающего код, и при каждом push
в `main` рабочим процессом
[Reproducible builds](https://github.com/laupernoe/dumbmonit/actions/workflows/reproducible.yml):
два раннера независимо собирают коммит, и задание завершается ошибкой, если
отличается хотя бы один файл, а отчёт [diffoscope](https://diffoscope.org/)
прикладывается как артефакт.

## Что делает сборку повторяемой

- **Зафиксированные зависимости.** `Cargo.lock` и `web/package-lock.json`
  включены в репозиторий; сборка использует `cargo build --locked` и `npm ci`.
- **Закреплённые инструменты.** Каждый базовый образ `Dockerfile` закреплён по
  дайджесту SHA-256: Node для веб-интерфейса, Rust на Alpine для сервера (его
  `gcc` и `musl-dev`, компилирующие SQLite, входят в этот образ), cargo-zigbuild
  для агентов (он фиксирует версию zig, который их компонует) и релиз
  VictoriaMetrics. Набор инструментов Rust для агентов закреплён через
  `RUST_TOOLCHAIN`.
- **Никакой даты сборки в результате.** Сборку определяет коммит, а не время
  сборки:
    - идентификатор версии веб-интерфейса (`version.name` в SvelteKit, по
      умолчанию текущее время) равен коммиту (`DUMBMONIT_BUILD`) или, иначе,
      `SOURCE_DATE_EPOCH`;
    - файлы, встроенные в сервер, не несут даты изменения (`rust-embed` с
      `deterministic-timestamps`);
    - заголовок PE агента для Windows получает фиксированную дату компоновки
      (`SOURCE_DATE_EPOCH=0` на этапе агента);
    - BuildKit записывает дату коммита в конфигурацию образа, его историю и, с
      `rewrite-timestamp=true`, в даты файлов слоёв.
- **Никаких путей машины в бинарных файлах.** Сборки выполняются в контейнерах с
  фиксированными путями (`/build`, `/usr/local/cargo`); сборка для macOS
  переопределяет свои пути с помощью `--remap-path-prefix`.
- **Фиксированный профиль сборки.** `Cargo.toml` фиксирует `lto`, одну единицу
  кодогенерации, `strip` и `panic = "abort"`.

## Как проверить самому

Вам нужен Docker с Buildx (BuildKit 0.13 или новее) и копия коммита или тега для
проверки. Передайте те же два входных значения, что и рабочий процесс релиза:
коммит и его дату:

```sh
git clone https://github.com/laupernoe/dumbmonit && cd dumbmonit
git checkout v0.9.1    # or any commit
export SOURCE_DATE_EPOCH=$(git log -1 --pretty=%ct)
set -- --build-arg DUMBMONIT_BUILD=$(git rev-parse HEAD) --platform linux/amd64 \
  --provenance=false --sbom=false

docker buildx build "$@" --target server-dist --output type=local,dest=out/server .
docker buildx build "$@" --target agent-dist  --output type=local,dest=out/agent .
sha256sum out/server/* out/agent/*
```

Соберите второй раз (на другой машине, в другом каталоге, с `--no-cache`) и
сравните контрольные суммы: они идентичны.

Чтобы сравнить с опубликованным релизом:

- **Бинарные файлы агента:** сравните с файлами `.sha256`, приложенными к релизу
  тега на GitHub.
- **Бинарный файл сервера:** извлеките его из опубликованного образа и сравните:

    ```sh
    docker create --name dm ghcr.io/laupernoe/dumbmonit:0.9.1
    docker cp dm:/dumbmonit published-dumbmonit && docker rm dm
    sha256sum published-dumbmonit out/server/dumbmonit
    ```

- **Образы:** соберите образ как архив OCI и сравните его слои с опубликованными
  (опубликованный образ также несёт метки, происхождение сборки и SBOM, которые
  меняют дайджест его манифеста, но не слои; образ агента: те же команды с
  `--target agent-image` и `ghcr.io/laupernoe/dumbmonit-agent`):

    ```sh
    docker buildx build "$@" --output type=oci,dest=image.tar,rewrite-timestamp=true .
    mkdir image && tar -xf image.tar -C image
    blob() { echo "image/blobs/sha256/${1#sha256:}"; }
    manifest=$(jq -r '.manifests[0].digest' image/index.json)
    config=$(jq -r '.config.digest' "$(blob "$manifest")")
    # Layers of the local build (uncompressed digests):
    jq -r '.rootfs.diff_ids[]' "$(blob "$config")"
    # Layers of the published image:
    docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.1 \
      --format '{{range .Image.RootFS.DiffIDs}}{{println .}}{{end}}'
    ```

Расхождение заслуживает issue: пожалуйста, откройте его, приложив выполненные
команды.

## Ограничения

- **Опубликованные образы до 0.9.1** собирались без `SOURCE_DATE_EPOCH`: их
  бинарные файлы и слои отличаются от пересборки. Сравнение локальной сборки с
  опубликованным образом ещё не выполнялось: первый релиз, собранный таким
  образом, — 0.9.1.
- **Агент для macOS** собирается на раннере GitHub с macOS (SDK Apple нельзя
  распространять, поэтому собрать его в контейнере невозможно). Он идентичен
  между двумя сборками на одной и той же версии macOS и Xcode, которые журнал
  релиза выводит (`xcodebuild -version`); другой Xcode может дать другой
  бинарный файл.
- **Время.** Базовые образы закреплены по дайджесту и остаются доступными, но
  индекс пакетов Alpine, используемый для `make` (инструмент сборки, не входящий
  в результат), npm и crates.io — онлайн-сервисы: пересборка спустя годы зависит
  от того, будут ли они всё ещё отдавать зафиксированные версии.
- **Без аргументов сборки** локальный `docker build .` без `DUMBMONIT_BUILD` и
  без `SOURCE_DATE_EPOCH` даёт идентификатор версии веб-интерфейса, взятый из
  текущего времени, и поэтому каждый раз другой бинарный файл сервера. Передавайте
  оба, как показано выше.

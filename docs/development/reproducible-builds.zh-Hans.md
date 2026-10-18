# 可复现构建 {#reproducible-builds}

同一个提交构建两次,得到的字节完全相同。对一个提交进行两次相互独立的
构建(全新的构建器、无缓存、不同的路径、不同的文件日期),以下内容
逐位完全一致:

- 服务器二进制文件(`dumbmonit`,静态 musl,嵌入了网页界面);
- 面向 Linux x86_64 和 aarch64、Windows x86_64 以及 FreeBSD x86_64 的
  agent 二进制文件;
- 嵌入服务器的网页界面文件;
- 服务器镜像和 agent 镜像,在以提交日期作为 `SOURCE_DATE_EPOCH` 构建
  时,按 OCI 清单摘要比较(每一层以及镜像配置);
- macOS agent 二进制文件,前提是在相同的 macOS 和 Xcode 版本上构建。

每个涉及代码的拉取请求以及每次推送到 `main` 时,
[Reproducible builds](https://github.com/laupernoe/dumbmonit/actions/workflows/reproducible.yml)
工作流都会进行检查:两个运行器各自独立构建该提交,只要有一个文件不同,
任务就会失败,并将 [diffoscope](https://diffoscope.org/) 报告作为产物
提供。

## 是什么让它可重复 {#what-makes-it-repeatable}

- **锁定的依赖。** `Cargo.lock` 和 `web/package-lock.json` 已提交;构建
  使用 `cargo build --locked` 和 `npm ci`。
- **固定的工具。** `Dockerfile` 中的每个基础镜像都按 SHA-256 摘要固定:
  用于网页界面的 Node、用于服务器的 Alpine 上的 Rust(编译 SQLite 的
  `gcc` 和 `musl-dev` 随该镜像提供)、用于 agent 的 cargo-zigbuild
  (它固定了负责链接的 zig 版本),以及 VictoriaMetrics 发行版。agent 的
  Rust 工具链由 `RUST_TOOLCHAIN` 固定。
- **输出中没有构建日期。** 一次构建由提交标识,而绝不是构建的时间:
    - 网页界面的版本标识符(SvelteKit 的 `version.name`,默认取当前
      时间)取自提交(`DUMBMONIT_BUILD`),否则取自 `SOURCE_DATE_EPOCH`;
    - 嵌入服务器的文件不带修改日期(`rust-embed` 的
      `deterministic-timestamps`);
    - Windows agent 的 PE 头采用固定的链接日期(agent 阶段中
      `SOURCE_DATE_EPOCH=0`);
    - BuildKit 把提交日期写入镜像配置、其历史记录,并在
      `rewrite-timestamp=true` 时写入各层的文件日期。
- **二进制文件中没有机器路径。** 构建在路径固定的容器中运行
  (`/build`、`/usr/local/cargo`);macOS 构建用
  `--remap-path-prefix` 重映射其路径。
- **固定的构建配置。** `Cargo.toml` 固定了 `lto`、单个代码生成单元、
  `strip` 和 `panic = "abort"`。

## 如何自己验证 {#how-to-check-it-yourself}

你需要安装了 Buildx 的 Docker(BuildKit 0.13 或更高版本),以及要检查
的提交或标签的检出。传入与发布工作流相同的两个输入,即提交及其日期:

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

再构建一次(换一台机器、换一个目录、使用 `--no-cache`)并比较校验和:
它们完全相同。

要与已发布的版本比较:

- **agent 二进制文件:** 与该标签的 GitHub 发布页附带的 `.sha256`
  文件比较。
- **服务器二进制文件:** 从已发布的镜像中取出并比较:

    ```sh
    docker create --name dm ghcr.io/laupernoe/dumbmonit:0.9.1
    docker cp dm:/dumbmonit published-dumbmonit && docker rm dm
    sha256sum published-dumbmonit out/server/dumbmonit
    ```

- **镜像:** 将镜像构建为 OCI 归档,并把它的各层与已发布的各层比较
  (已发布的镜像还带有标签、构建来源证明和 SBOM,它们会改变清单摘要,
  但不会改变各层;agent 镜像:同样的命令,换成
  `--target agent-image` 和 `ghcr.io/laupernoe/dumbmonit-agent`):

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

不一致值得提交一个 issue:请附上你运行的命令。

## 局限 {#limits}

- **0.9.1 之前发布的镜像**在构建时没有设置 `SOURCE_DATE_EPOCH`:它们的
  二进制文件和各层与重新构建的结果不同。本地构建与已发布镜像的比较
  尚未进行:第一个以这种方式构建的版本是 0.9.1。
- **macOS agent** 在 GitHub 的 macOS 运行器上构建(Apple 的 SDK 不得
  再分发,因此无法在容器中构建)。在相同的 macOS 和 Xcode 上的两次
  构建结果一致,发布日志会打印这一信息(`xcodebuild -version`);换用
  其他 Xcode 可能得到不同的二进制文件。
- **时间。** 基础镜像按摘要固定且保持可用,但用于 `make`(构建工具,
  不属于输出)的 Alpine 软件包索引、npm 和 crates.io 都是在线服务:
  多年之后的重新构建取决于它们是否仍然提供被锁定的版本。
- **不带构建参数时**,本地的 `docker build .` 如果没有
  `DUMBMONIT_BUILD` 和 `SOURCE_DATE_EPOCH`,网页界面的版本标识符会取自
  当前时间,因此每次得到的服务器二进制文件都不同。请像上面那样传入
  两者。

# Builds reproduzíveis {#reproducible-builds}

Construir o mesmo commit duas vezes dá os mesmos bytes. Dois builds
independentes de um commit (construtores novos, sem cache, caminhos
diferentes, datas de arquivo diferentes) produzem, bit a bit, idênticos:

- o binário do servidor (`dumbmonit`, musl estático, com a interface web
  embutida);
- os binários do agente para Linux x86_64 e aarch64, Windows x86_64 e FreeBSD
  x86_64;
- os arquivos da interface web embutidos no servidor;
- a imagem do servidor e a imagem do agente, comparadas pelo digest do
  manifesto OCI (cada camada e a configuração da imagem), quando construídas
  com a data do commit como `SOURCE_DATE_EPOCH`;
- os binários do agente para macOS, quando construídos na mesma versão de macOS
  e Xcode.

Isso é verificado em cada pull request que toca o código e em cada push para
`main` pelo workflow
[Reproducible builds](https://github.com/laupernoe/dumbmonit/actions/workflows/reproducible.yml):
dois runners constroem o commit de forma independente e o job falha se um único
arquivo difere, com um relatório do [diffoscope](https://diffoscope.org/) como
artefato.

## O que o torna repetível {#what-makes-it-repeatable}

- **Dependências travadas.** `Cargo.lock` e `web/package-lock.json` estão no
  repositório; o build usa `cargo build --locked` e `npm ci`.
- **Ferramentas fixadas.** Toda imagem base do `Dockerfile` é fixada pelo
  digest SHA-256: Node para a interface web, Rust sobre Alpine para o servidor
  (seu `gcc` e `musl-dev`, que compilam o SQLite, vêm com essa imagem),
  cargo-zigbuild para os agentes (ele fixa a versão do zig, que os liga) e a
  release do VictoriaMetrics. A toolchain Rust dos agentes é fixada com
  `RUST_TOOLCHAIN`.
- **Nenhuma data de build na saída.** O commit identifica um build, nunca o
  momento do build:
    - o identificador de versão da interface web (o `version.name` do
      SvelteKit, que por padrão é a hora atual) é o commit
      (`DUMBMONIT_BUILD`) ou, na falta, `SOURCE_DATE_EPOCH`;
    - os arquivos embutidos no servidor não carregam data de modificação
      (`rust-embed` com `deterministic-timestamps`);
    - o cabeçalho PE do agente Windows recebe uma data de link fixa
      (`SOURCE_DATE_EPOCH=0` no estágio do agente);
    - o BuildKit grava a data do commit na configuração da imagem, em seu
      histórico e, com `rewrite-timestamp=true`, nas datas dos arquivos das
      camadas.
- **Nenhum caminho da máquina nos binários.** Os builds rodam em contêineres
  com caminhos fixos (`/build`, `/usr/local/cargo`); o build do macOS remapeia
  seus caminhos com `--remap-path-prefix`.
- **Perfil de build fixo.** O `Cargo.toml` fixa `lto`, uma unidade de geração
  de código, `strip` e `panic = "abort"`.

## Como verificar você mesmo {#how-to-check-it-yourself}

Você precisa do Docker com Buildx (BuildKit 0.13 ou posterior) e de um
checkout do commit ou da tag a verificar. Passe as mesmas duas entradas do
workflow de release, o commit e sua data:

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

Construa uma segunda vez (outra máquina, outro diretório, `--no-cache`) e
compare os checksums: eles são idênticos.

Para comparar com uma versão publicada:

- **Binários do agente:** compare com os arquivos `.sha256` anexados à release
  do GitHub da tag.
- **Binário do servidor:** retire-o da imagem publicada e compare:

    ```sh
    docker create --name dm ghcr.io/laupernoe/dumbmonit:0.9.1
    docker cp dm:/dumbmonit published-dumbmonit && docker rm dm
    sha256sum published-dumbmonit out/server/dumbmonit
    ```

- **Imagens:** construa a imagem como um arquivo OCI e compare suas camadas com
  as publicadas (a imagem publicada também traz rótulos, uma proveniência de
  build e um SBOM, que mudam o digest do manifesto mas não as camadas; imagem
  do agente: os mesmos comandos com `--target agent-image` e
  `ghcr.io/laupernoe/dumbmonit-agent`):

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

Uma divergência merece uma issue: por favor, abra uma com os comandos que você
executou.

## Limites {#limits}

- **Imagens publicadas antes da 0.9.1** foram construídas sem
  `SOURCE_DATE_EPOCH`: seus binários e camadas diferem de uma reconstrução. A
  comparação de um build local com uma imagem publicada ainda não foi
  executada: a primeira release construída assim é a 0.9.1.
- **O agente macOS** é construído em um runner macOS do GitHub (o SDK da Apple
  não pode ser redistribuído, então não dá para construí-lo em um contêiner). Ele
  é idêntico entre dois builds no mesmo macOS e Xcode, que o log da release
  imprime (`xcodebuild -version`); outro Xcode pode dar outro binário.
- **Tempo.** As imagens base são fixadas por digest e continuam disponíveis,
  mas o índice de pacotes do Alpine usado para o `make` (uma ferramenta de
  build, que não faz parte da saída), o npm e o crates.io são serviços online:
  uma reconstrução anos depois depende de eles ainda servirem as versões
  travadas.
- **Sem os argumentos de build**, um `docker build .` local sem
  `DUMBMONIT_BUILD` e sem `SOURCE_DATE_EPOCH` dá um identificador de versão da
  interface web tirado da hora atual e, portanto, um binário do servidor
  diferente a cada vez. Passe os dois, como acima.

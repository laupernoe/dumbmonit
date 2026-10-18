# Compilações reprodutíveis {#reproducible-builds}

Compilar o mesmo commit duas vezes dá os mesmos bytes. Duas compilações
independentes de um commit (construtores novos, sem cache, caminhos
diferentes, datas de ficheiros diferentes) produzem, bit a bit, idênticos:

- o binário do servidor (`dumbmonit`, musl estático, com a interface web
  embebida);
- os binários do agente para Linux x86_64 e aarch64, Windows x86_64 e FreeBSD
  x86_64;
- os ficheiros da interface web embebidos no servidor;
- a imagem do servidor e a imagem do agente, comparadas pelo digest do
  manifesto OCI (todas as camadas e a configuração da imagem), quando
  compiladas com a data do commit como `SOURCE_DATE_EPOCH`;
- os binários do agente para macOS, quando compilados na mesma versão de macOS
  e Xcode.

Isto é verificado em cada pull request que toca no código e em cada push para
`main` pelo fluxo de trabalho
[Reproducible builds](https://github.com/laupernoe/dumbmonit/actions/workflows/reproducible.yml):
dois executores compilam o commit de forma independente e o trabalho falha se
um único ficheiro diferir, com um relatório do [diffoscope](https://diffoscope.org/)
como artefacto.

## O que o torna repetível {#what-makes-it-repeatable}

- **Dependências fixadas.** `Cargo.lock` e `web/package-lock.json` estão
  versionados; a compilação usa `cargo build --locked` e `npm ci`.
- **Ferramentas fixadas.** Cada imagem base do `Dockerfile` está fixada por
  digest SHA-256: Node para a interface web, Rust em Alpine para o servidor (o
  seu `gcc` e `musl-dev`, que compilam o SQLite, vêm nessa imagem),
  cargo-zigbuild para os agentes (fixa a versão do zig, que os liga), e a
  versão do VictoriaMetrics. A toolchain Rust dos agentes está fixada com
  `RUST_TOOLCHAIN`.
- **Nenhuma data de compilação no resultado.** O commit identifica uma
  compilação, nunca a hora da compilação:
    - o identificador de versão da interface web (`version.name` do SvelteKit,
      que por predefinição é a hora atual) é o commit (`DUMBMONIT_BUILD`), ou
      então `SOURCE_DATE_EPOCH`;
    - os ficheiros embebidos no servidor não têm data de modificação
      (`rust-embed` com `deterministic-timestamps`);
    - o cabeçalho PE do agente Windows recebe uma data de ligação fixa
      (`SOURCE_DATE_EPOCH=0` na fase do agente);
    - o BuildKit escreve a data do commit na configuração da imagem, no seu
      histórico e, com `rewrite-timestamp=true`, nas datas dos ficheiros das
      camadas.
- **Nenhum caminho da máquina nos binários.** As compilações correm em
  contentores com caminhos fixos (`/build`, `/usr/local/cargo`); a compilação
  para macOS remapeia os seus caminhos com `--remap-path-prefix`.
- **Perfil de compilação fixo.** O `Cargo.toml` fixa `lto`, uma unidade de
  geração de código, `strip` e `panic = "abort"`.

## Como verificar por si {#how-to-check-it-yourself}

Precisa de Docker com Buildx (BuildKit 0.13 ou posterior) e de um checkout do
commit ou etiqueta a verificar. Passe as mesmas duas entradas do fluxo de
trabalho de versões, o commit e a sua data:

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

Compile uma segunda vez (outra máquina, outro diretório, `--no-cache`) e
compare as somas de verificação: são idênticas.

Para comparar com uma versão publicada:

- **Binários do agente:** compare com os ficheiros `.sha256` anexados à versão
  do GitHub da etiqueta.
- **Binário do servidor:** extraia-o da imagem publicada e compare:

    ```sh
    docker create --name dm ghcr.io/laupernoe/dumbmonit:0.9.1
    docker cp dm:/dumbmonit published-dumbmonit && docker rm dm
    sha256sum published-dumbmonit out/server/dumbmonit
    ```

- **Imagens:** compile a imagem como arquivo OCI e compare as suas camadas com
  as publicadas (a imagem publicada também leva etiquetas, uma proveniência de
  compilação e um SBOM, que mudam o digest do seu manifesto mas não as suas
  camadas; imagem do agente: os mesmos comandos com `--target agent-image` e
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

Uma divergência merece uma issue: abra uma com os comandos que executou.

## Limites {#limits}

- **As imagens publicadas antes da 0.9.1** foram compiladas sem
  `SOURCE_DATE_EPOCH`: os seus binários e camadas diferem de uma recompilação.
  A comparação de uma compilação local com uma imagem publicada ainda não foi
  executada: a primeira versão compilada assim é a 0.9.1.
- **O agente macOS** é compilado num executor macOS do GitHub (o SDK da Apple
  não pode ser redistribuído, por isso não pode ser compilado num contentor).
  É idêntico entre duas compilações no mesmo macOS e Xcode, que o registo da
  versão imprime (`xcodebuild -version`); outro Xcode pode dar outro binário.
- **Tempo.** As imagens base estão fixadas por digest e continuam disponíveis,
  mas o índice de pacotes Alpine usado para o `make` (uma ferramenta de
  compilação, que não faz parte do resultado), o npm e o crates.io são
  serviços online: uma recompilação anos depois depende de eles ainda servirem
  as versões fixadas.
- **Sem os argumentos de compilação**, um `docker build .` local sem
  `DUMBMONIT_BUILD` nem `SOURCE_DATE_EPOCH` dá um identificador de versão da
  interface web tirado da hora atual e, portanto, um binário do servidor
  diferente de cada vez. Passe ambos, como acima.

# Build riproducibili

Compilare due volte lo stesso commit dà gli stessi byte. Due build
indipendenti di un commit (builder nuovi, senza cache, percorsi diversi, date
dei file diverse) producono bit per bit identici:

- il binario del server (`dumbmonit`, musl statico, con la web UI incorporata);
- i binari dell'agente per Linux x86_64 e aarch64, Windows x86_64 e FreeBSD
  x86_64;
- i file della web UI incorporati nel server;
- l'immagine del server e l'immagine dell'agente, confrontate tramite il
  digest del manifest OCI (ogni layer e la configurazione dell'immagine),
  quando compilate con la data del commit come `SOURCE_DATE_EPOCH`;
- i binari dell'agente macOS, quando compilati sulla stessa versione di macOS
  e Xcode.

Questo viene verificato a ogni pull request che tocca il codice e a ogni push
su `main` dal workflow
[Reproducible builds](https://github.com/laupernoe/dumbmonit/actions/workflows/reproducible.yml):
due runner compilano il commit in modo indipendente e il job fallisce se un
solo file differisce, con un rapporto [diffoscope](https://diffoscope.org/)
come artefatto.

## Cosa lo rende ripetibile

- **Dipendenze bloccate.** `Cargo.lock` e `web/package-lock.json` sono nel
  repository; la build usa `cargo build --locked` e `npm ci`.
- **Strumenti fissati.** Ogni immagine di base del `Dockerfile` è fissata
  tramite digest SHA-256: Node per la web UI, Rust su Alpine per il server (i
  suoi `gcc` e `musl-dev`, che compilano SQLite, arrivano con quell'immagine),
  cargo-zigbuild per gli agenti (fissa la versione di zig, che li collega) e
  la release di VictoriaMetrics. La toolchain Rust degli agenti è fissata con
  `RUST_TOOLCHAIN`.
- **Nessuna data di build nell'output.** Il commit identifica una build, mai
  l'ora della build:
    - l'identificatore di versione della web UI (`version.name` di SvelteKit,
      che per impostazione predefinita è l'ora corrente) è il commit
      (`DUMBMONIT_BUILD`), oppure `SOURCE_DATE_EPOCH`;
    - i file incorporati nel server non portano alcuna data di modifica
      (`rust-embed` con `deterministic-timestamps`);
    - l'header PE dell'agente Windows riceve una data di link fissa
      (`SOURCE_DATE_EPOCH=0` nello stage dell'agente);
    - BuildKit scrive la data del commit nella configurazione dell'immagine,
      nella sua cronologia e, con `rewrite-timestamp=true`, nelle date dei
      file dei layer.
- **Nessun percorso della macchina nei binari.** Le build girano in container
  con percorsi fissi (`/build`, `/usr/local/cargo`); la build macOS rimappa i
  suoi percorsi con `--remap-path-prefix`.
- **Profilo di build fisso.** `Cargo.toml` fissa `lto`, un solo codegen unit,
  `strip` e `panic = "abort"`.

## Come verificarlo da solo

Ti servono Docker con Buildx (BuildKit 0.13 o successivo) e un checkout del
commit o del tag da verificare. Passa gli stessi due input del workflow di
release, il commit e la sua data:

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

Compila una seconda volta (un'altra macchina, un'altra directory,
`--no-cache`) e confronta i checksum: sono identici.

Per confrontare con una release pubblicata:

- **Binari dell'agente:** confronta con i file `.sha256` allegati alla release
  GitHub del tag.
- **Binario del server:** estrailo dall'immagine pubblicata e confronta:

    ```sh
    docker create --name dm ghcr.io/laupernoe/dumbmonit:0.9.1
    docker cp dm:/dumbmonit published-dumbmonit && docker rm dm
    sha256sum published-dumbmonit out/server/dumbmonit
    ```

- **Immagini:** compila l'immagine come archivio OCI e confronta i suoi layer
  con quelli pubblicati (l'immagine pubblicata porta anche etichette, una
  provenienza di build e un SBOM, che cambiano il digest del suo manifest ma
  non i suoi layer; immagine dell'agente: stessi comandi con
  `--target agent-image` e `ghcr.io/laupernoe/dumbmonit-agent`):

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

Una discrepanza merita una segnalazione: aprine una con i comandi che hai
eseguito.

## Limiti

- **Le immagini pubblicate prima della 0.9.1** sono state compilate senza
  `SOURCE_DATE_EPOCH`: i loro binari e layer differiscono da una ricompilazione.
  Il confronto di una build locale con un'immagine pubblicata non è ancora
  stato eseguito: la prima release compilata in questo modo è la 0.9.1.
- **L'agente macOS** è compilato su un runner macOS di GitHub (l'SDK di Apple
  non può essere ridistribuito, quindi non si può compilare in un container).
  È identico tra due build sulla stessa versione di macOS e Xcode, che il log
  della release stampa (`xcodebuild -version`); un altro Xcode può dare un
  altro binario.
- **Il tempo.** Le immagini di base sono fissate tramite digest e restano
  disponibili, ma l'indice dei pacchetti Alpine usato per `make` (uno
  strumento di build, non parte dell'output), npm e crates.io sono servizi
  online: una ricompilazione anni dopo dipende dal fatto che servano ancora le
  versioni bloccate.
- **Senza gli argomenti di build**, un `docker build .` locale senza
  `DUMBMONIT_BUILD` né `SOURCE_DATE_EPOCH` dà un identificatore di versione
  della web UI preso dall'ora corrente, e quindi un binario del server diverso
  ogni volta. Passali entrambi, come sopra.

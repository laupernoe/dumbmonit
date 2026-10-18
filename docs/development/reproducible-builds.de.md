# Reproduzierbare Builds {#reproducible-builds}

Derselbe Commit, zweimal gebaut, ergibt dieselben Bytes. Zwei unabhängige Builds
eines Commits (frische Builder, kein Cache, andere Pfade, andere Dateidaten)
erzeugen bitgenau identisch:

- das Server-Binary (`dumbmonit`, statisch mit musl, die Web-Oberfläche
  eingebettet);
- die Agent-Binaries für Linux x86_64 und aarch64, Windows x86_64 und FreeBSD
  x86_64;
- die in den Server eingebetteten Dateien der Web-Oberfläche;
- das Server-Image und das Agent-Image, verglichen über den OCI-Manifest-Digest
  (jede Schicht und die Image-Konfiguration), wenn mit dem Commit-Datum als
  `SOURCE_DATE_EPOCH` gebaut;
- die macOS-Agent-Binaries, wenn auf derselben macOS- und Xcode-Version gebaut.

Das wird bei jedem Pull Request, der den Code berührt, und bei jedem Push auf
`main` vom Workflow
[Reproducible builds](https://github.com/laupernoe/dumbmonit/actions/workflows/reproducible.yml)
geprüft: Zwei Runner bauen den Commit unabhängig voneinander, und der Job
schlägt fehl, wenn sich auch nur eine Datei unterscheidet, mit einem Bericht von
[diffoscope](https://diffoscope.org/) als Artefakt.

## Was den Build wiederholbar macht {#what-makes-it-repeatable}

- **Fixierte Abhängigkeiten.** `Cargo.lock` und `web/package-lock.json` sind
  eingecheckt; der Build nutzt `cargo build --locked` und `npm ci`.
- **Fixierte Werkzeuge.** Jedes Basis-Image des `Dockerfile` ist per SHA-256-Digest
  fixiert: Node für die Web-Oberfläche, Rust auf Alpine für den Server (dessen
  `gcc` und `musl-dev`, die SQLite kompilieren, kommen mit diesem Image),
  cargo-zigbuild für die Agenten (es legt die Version von zig fest, das sie
  linkt) und das VictoriaMetrics-Release. Die Rust-Toolchain der Agenten ist mit
  `RUST_TOOLCHAIN` fixiert.
- **Kein Build-Datum in der Ausgabe.** Der Commit identifiziert einen Build, nie
  der Zeitpunkt des Builds:
    - der Versionsbezeichner der Web-Oberfläche (`version.name` von SvelteKit, der
      standardmäßig die aktuelle Zeit ist) ist der Commit (`DUMBMONIT_BUILD`),
      sonst `SOURCE_DATE_EPOCH`;
    - die in den Server eingebetteten Dateien tragen kein Änderungsdatum
      (`rust-embed` mit `deterministic-timestamps`);
    - der PE-Header des Windows-Agenten erhält ein festes Link-Datum
      (`SOURCE_DATE_EPOCH=0` in der Agent-Stufe);
    - BuildKit schreibt das Commit-Datum in die Image-Konfiguration, ihre Historie
      und, mit `rewrite-timestamp=true`, in die Dateidaten der Schichten.
- **Keine Maschinenpfade in den Binaries.** Die Builds laufen in Containern mit
  festen Pfaden (`/build`, `/usr/local/cargo`); der macOS-Build bildet seine Pfade
  mit `--remap-path-prefix` um.
- **Festes Build-Profil.** `Cargo.toml` legt `lto`, eine Codegen-Unit, `strip` und
  `panic = "abort"` fest.

## So prüfen Sie es selbst {#how-to-check-it-yourself}

Sie brauchen Docker mit Buildx (BuildKit 0.13 oder neuer) und einen Checkout des zu
prüfenden Commits oder Tags. Übergeben Sie dieselben zwei Eingaben wie der
Release-Workflow, den Commit und sein Datum:

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

Bauen Sie ein zweites Mal (andere Maschine, anderes Verzeichnis, `--no-cache`) und
vergleichen Sie die Prüfsummen: Sie sind identisch.

So vergleichen Sie mit einem veröffentlichten Release:

- **Agent-Binaries:** Vergleichen Sie mit den `.sha256`-Dateien, die am
  GitHub-Release des Tags hängen.
- **Server-Binary:** Nehmen Sie es aus dem veröffentlichten Image und vergleichen
  Sie:

    ```sh
    docker create --name dm ghcr.io/laupernoe/dumbmonit:0.9.1
    docker cp dm:/dumbmonit published-dumbmonit && docker rm dm
    sha256sum published-dumbmonit out/server/dumbmonit
    ```

- **Images:** Bauen Sie das Image als OCI-Archiv und vergleichen Sie seine Schichten
  mit den veröffentlichten (das veröffentlichte Image trägt außerdem Labels, eine
  Build-Provenance und eine SBOM, die seinen Manifest-Digest ändern, nicht aber
  seine Schichten; Agent-Image: dieselben Befehle mit `--target agent-image` und
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

Eine Abweichung ist ein Issue wert: Bitte eröffnen Sie eines mit den Befehlen, die
Sie ausgeführt haben.

## Grenzen {#limits}

- **Vor 0.9.1 veröffentlichte Images** wurden ohne `SOURCE_DATE_EPOCH` gebaut: Ihre
  Binaries und Schichten unterscheiden sich von einem Neubau. Der Vergleich eines
  lokalen Builds mit einem veröffentlichten Image wurde noch nicht durchgeführt: Das
  erste so gebaute Release ist 0.9.1.
- **Der macOS-Agent** wird auf einem GitHub-macOS-Runner gebaut (Apples SDK darf
  nicht weiterverteilt werden, er lässt sich also nicht in einem Container bauen).
  Er ist zwischen zwei Builds auf derselben macOS- und Xcode-Version identisch, was
  das Release-Log ausgibt (`xcodebuild -version`); ein anderes Xcode kann ein anderes
  Binary ergeben.
- **Zeit.** Die Basis-Images sind per Digest fixiert und bleiben verfügbar, aber der
  Alpine-Paketindex für `make` (ein Build-Werkzeug, nicht Teil der Ausgabe), npm und
  crates.io sind Online-Dienste: Ein Neubau Jahre später hängt davon ab, dass sie die
  fixierten Versionen noch ausliefern.
- **Ohne die Build-Argumente** ergibt ein lokales `docker build .` ohne
  `DUMBMONIT_BUILD` und ohne `SOURCE_DATE_EPOCH` einen Versionsbezeichner der
  Web-Oberfläche aus der aktuellen Zeit und damit jedes Mal ein anderes
  Server-Binary. Übergeben Sie beide, wie oben.

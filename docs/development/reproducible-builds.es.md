# Compilaciones reproducibles {#reproducible-builds}

Compilar dos veces el mismo commit da los mismos bytes. Dos compilaciones
independientes de un commit (compiladores nuevos, sin caché, con rutas y fechas
de archivo distintas) producen, bit a bit, idénticos:

- el binario del servidor (`dumbmonit`, musl estático, con la interfaz web
  incrustada);
- los binarios del agente para Linux x86_64 y aarch64, Windows x86_64 y FreeBSD
  x86_64;
- los archivos de la interfaz web incrustados en el servidor;
- la imagen del servidor y la imagen del agente, comparadas por el digest del
  manifiesto OCI (cada capa y la configuración de la imagen), cuando se compilan
  con la fecha del commit como `SOURCE_DATE_EPOCH`;
- los binarios del agente para macOS, cuando se compilan con la misma versión de
  macOS y de Xcode.

Esto se comprueba en cada pull request que toca el código y en cada push a
`main` con el flujo de trabajo
[Reproducible builds](https://github.com/laupernoe/dumbmonit/actions/workflows/reproducible.yml):
dos ejecutores compilan el commit de forma independiente y el trabajo falla si un
solo archivo difiere, con un informe de [diffoscope](https://diffoscope.org/)
como artefacto.

## Qué lo hace repetible {#what-makes-it-repeatable}

- **Dependencias bloqueadas.** `Cargo.lock` y `web/package-lock.json` están en el
  repositorio; la compilación usa `cargo build --locked` y `npm ci`.
- **Herramientas fijadas.** Cada imagen base del `Dockerfile` está fijada por su
  digest SHA-256: Node para la interfaz web, Rust sobre Alpine para el servidor
  (su `gcc` y `musl-dev`, que compilan SQLite, vienen con esa imagen),
  cargo-zigbuild para los agentes (fija la versión de zig, que los enlaza) y la
  versión de VictoriaMetrics. La cadena de herramientas de Rust de los agentes se
  fija con `RUST_TOOLCHAIN`.
- **Ninguna fecha de compilación en el resultado.** El commit identifica una
  compilación, nunca la hora de la compilación:
    - el identificador de versión de la interfaz web (`version.name` de
      SvelteKit, que por defecto es la hora actual) es el commit
      (`DUMBMONIT_BUILD`) o, si no, `SOURCE_DATE_EPOCH`;
    - los archivos incrustados en el servidor no llevan fecha de modificación
      (`rust-embed` con `deterministic-timestamps`);
    - la cabecera PE del agente de Windows recibe una fecha de enlazado fija
      (`SOURCE_DATE_EPOCH=0` en la etapa del agente);
    - BuildKit escribe la fecha del commit en la configuración de la imagen, en
      su historial y, con `rewrite-timestamp=true`, en las fechas de archivo de
      las capas.
- **Ninguna ruta de la máquina en los binarios.** Las compilaciones se ejecutan
  en contenedores con rutas fijas (`/build`, `/usr/local/cargo`); la compilación
  de macOS reasigna sus rutas con `--remap-path-prefix`.
- **Perfil de compilación fijo.** `Cargo.toml` fija `lto`, una unidad de
  generación de código, `strip` y `panic = "abort"`.

## Cómo comprobarlo tú mismo {#how-to-check-it-yourself}

Necesitas Docker con Buildx (BuildKit 0.13 o posterior) y una copia del commit o
la etiqueta que quieres comprobar. Pasa las mismas dos entradas que el flujo de
trabajo de versiones, el commit y su fecha:

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

Compila una segunda vez (otra máquina, otro directorio, `--no-cache`) y compara
las sumas de verificación: son idénticas.

Para comparar con una versión publicada:

- **Binarios del agente:** compara con los archivos `.sha256` adjuntos a la
  versión de GitHub de la etiqueta.
- **Binario del servidor:** sácalo de la imagen publicada y compara:

    ```sh
    docker create --name dm ghcr.io/laupernoe/dumbmonit:0.9.1
    docker cp dm:/dumbmonit published-dumbmonit && docker rm dm
    sha256sum published-dumbmonit out/server/dumbmonit
    ```

- **Imágenes:** compila la imagen como archivo OCI y compara sus capas con las
  publicadas (la imagen publicada también lleva etiquetas, una procedencia de
  compilación y un SBOM, que cambian el digest de su manifiesto pero no sus
  capas; imagen del agente: los mismos comandos con `--target agent-image` y
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

Una discrepancia merece una incidencia: abre una con los comandos que ejecutaste.

## Límites {#limits}

- **Las imágenes publicadas antes de la 0.9.1** se compilaron sin
  `SOURCE_DATE_EPOCH`: sus binarios y capas difieren de una recompilación. La
  comparación de una compilación local con una imagen publicada aún no se ha
  hecho: la primera versión compilada así es la 0.9.1.
- **El agente de macOS** se compila en un ejecutor macOS de GitHub (el SDK de
  Apple no se puede redistribuir, así que no se puede compilar en un
  contenedor). Es idéntico entre dos compilaciones con el mismo macOS y Xcode,
  que el registro de la versión imprime (`xcodebuild -version`); otro Xcode
  puede dar otro binario.
- **El tiempo.** Las imágenes base están fijadas por digest y siguen
  disponibles, pero el índice de paquetes de Alpine usado para `make` (una
  herramienta de compilación, no parte del resultado), npm y crates.io son
  servicios en línea: una recompilación años después depende de que sigan
  sirviendo las versiones bloqueadas.
- **Sin los argumentos de compilación**, un `docker build .` local sin
  `DUMBMONIT_BUILD` ni `SOURCE_DATE_EPOCH` da un identificador de versión de la
  interfaz web tomado de la hora actual y, por tanto, un binario del servidor
  distinto cada vez. Pasa ambos, como arriba.

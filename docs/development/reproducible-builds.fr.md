# Builds reproductibles

Construire deux fois le même commit donne les mêmes octets. Deux builds
indépendants d'un commit (builders neufs, sans cache, chemins différents, dates
de fichiers différentes) produisent des résultats identiques bit à bit :

- le binaire du serveur (`dumbmonit`, musl statique, interface web embarquée) ;
- les binaires de l'agent pour Linux x86_64 et aarch64, Windows x86_64 et
  FreeBSD x86_64 ;
- les fichiers de l'interface web embarqués dans le serveur ;
- l'image du serveur et l'image de l'agent, comparées par le digest du manifeste
  OCI (chaque couche et la configuration de l'image), lorsqu'elles sont
  construites avec la date du commit comme `SOURCE_DATE_EPOCH` ;
- les binaires de l'agent macOS, lorsqu'ils sont construits sur la même version
  de macOS et de Xcode.

Cela est vérifié à chaque pull request qui touche au code et à chaque push sur
`main` par le workflow
[Reproducible builds](https://github.com/laupernoe/dumbmonit/actions/workflows/reproducible.yml) :
deux runners construisent le commit indépendamment et le job échoue dès qu'un
seul fichier diffère, avec un rapport [diffoscope](https://diffoscope.org/)
en artefact.

## Ce qui rend le build répétable

- **Dépendances verrouillées.** `Cargo.lock` et `web/package-lock.json` sont
  versionnés ; le build utilise `cargo build --locked` et `npm ci`.
- **Outils épinglés.** Chaque image de base du `Dockerfile` est épinglée par
  digest SHA-256 : Node pour l'interface web, Rust sur Alpine pour le serveur
  (son `gcc` et `musl-dev`, qui compilent SQLite, viennent avec cette image),
  cargo-zigbuild pour les agents (il fixe la version de zig, qui les lie) et la
  version de VictoriaMetrics. La toolchain Rust des agents est épinglée avec
  `RUST_TOOLCHAIN`.
- **Aucune date de build dans le résultat.** Le commit identifie un build,
  jamais l'heure du build :
    - l'identifiant de version de l'interface web (`version.name` de SvelteKit,
      qui vaut par défaut l'heure courante) est le commit (`DUMBMONIT_BUILD`),
      ou à défaut `SOURCE_DATE_EPOCH` ;
    - les fichiers embarqués dans le serveur ne portent aucune date de
      modification (`rust-embed` avec `deterministic-timestamps`) ;
    - l'en-tête PE de l'agent Windows reçoit une date d'édition de liens fixe
      (`SOURCE_DATE_EPOCH=0` dans l'étape de l'agent) ;
    - BuildKit écrit la date du commit dans la configuration de l'image, son
      historique et, avec `rewrite-timestamp=true`, les dates de fichiers des
      couches.
- **Aucun chemin de machine dans les binaires.** Les builds s'exécutent dans des
  conteneurs aux chemins fixes (`/build`, `/usr/local/cargo`) ; le build macOS
  remappe ses chemins avec `--remap-path-prefix`.
- **Profil de build fixe.** `Cargo.toml` fixe `lto`, une seule unité de
  génération de code, `strip` et `panic = "abort"`.

## Comment le vérifier vous-même

Il vous faut Docker avec Buildx (BuildKit 0.13 ou plus récent) et une copie du
commit ou du tag à vérifier. Passez les deux mêmes entrées que le workflow de
publication, le commit et sa date :

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

Construisez une seconde fois (autre machine, autre répertoire, `--no-cache`) et
comparez les sommes de contrôle : elles sont identiques.

Pour comparer avec une version publiée :

- **Binaires de l'agent :** comparez avec les fichiers `.sha256` joints à la
  version GitHub du tag.
- **Binaire du serveur :** extrayez-le de l'image publiée et comparez :

    ```sh
    docker create --name dm ghcr.io/laupernoe/dumbmonit:0.9.1
    docker cp dm:/dumbmonit published-dumbmonit && docker rm dm
    sha256sum published-dumbmonit out/server/dumbmonit
    ```

- **Images :** construisez l'image sous forme d'archive OCI et comparez ses
  couches avec celles de l'image publiée (l'image publiée porte aussi des
  étiquettes, une provenance de build et un SBOM, qui changent le digest de son
  manifeste mais pas ses couches ; image de l'agent : mêmes commandes avec
  `--target agent-image` et `ghcr.io/laupernoe/dumbmonit-agent`) :

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

Un écart mérite un ticket : ouvrez-en un en indiquant les commandes exécutées.

## Limites

- **Les images publiées avant la 0.9.1** ont été construites sans
  `SOURCE_DATE_EPOCH` : leurs binaires et leurs couches diffèrent d'une
  reconstruction. La comparaison d'un build local avec une image publiée n'a pas
  encore été exécutée : la première version construite ainsi est la 0.9.1.
- **L'agent macOS** est construit sur un runner macOS de GitHub (le SDK d'Apple
  ne peut pas être redistribué, il ne peut donc pas être construit dans un
  conteneur). Il est identique entre deux builds sur les mêmes versions de macOS
  et de Xcode, que le journal de la version affiche (`xcodebuild -version`) ; un
  autre Xcode peut donner un autre binaire.
- **Le temps.** Les images de base sont épinglées par digest et restent
  disponibles, mais l'index de paquets Alpine utilisé pour `make` (un outil de
  build, qui ne fait pas partie du résultat), npm et crates.io sont des services
  en ligne : une reconstruction des années plus tard dépend de leur capacité à
  servir encore les versions verrouillées.
- **Sans les arguments de build**, un `docker build .` local sans
  `DUMBMONIT_BUILD` ni `SOURCE_DATE_EPOCH` donne un identifiant de version de
  l'interface web tiré de l'heure courante, et donc un binaire de serveur
  différent à chaque fois. Passez les deux, comme ci-dessus.

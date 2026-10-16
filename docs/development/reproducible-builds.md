# Reproducible builds

Building the same commit twice gives the same bytes. Two independent builds of
a commit (fresh builders, no cache, different paths, different file dates)
produce bit-for-bit identical:

- the server binary (`dumbmonit`, static musl, the web UI embedded);
- the agent binaries for Linux x86_64 and aarch64, Windows x86_64 and FreeBSD
  x86_64;
- the web UI files embedded in the server;
- the server image and the agent image, compared by OCI manifest digest
  (every layer and the image configuration), when built with the commit date
  as `SOURCE_DATE_EPOCH`;
- the macOS agent binaries, when built on the same macOS and Xcode version.

This is checked on every pull request that touches the code and on every push
to `main` by the
[Reproducible builds](https://github.com/laupernoe/dumbmonit/actions/workflows/reproducible.yml)
workflow: two runners build the commit independently and the job fails if a
single file differs, with a [diffoscope](https://diffoscope.org/) report as an
artifact.

## What makes it repeatable

- **Locked dependencies.** `Cargo.lock` and `web/package-lock.json` are
  committed; the build uses `cargo build --locked` and `npm ci`.
- **Pinned tools.** Every base image of the `Dockerfile` is pinned by SHA-256
  digest: Node for the web UI, Rust on Alpine for the server (its `gcc` and
  `musl-dev`, which compile SQLite, come with that image), cargo-zigbuild for
  the agents (it fixes the version of zig, which links them), and the
  VictoriaMetrics release. The agents' Rust toolchain is pinned with
  `RUST_TOOLCHAIN`.
- **No build date in the output.** The commit identifies a build, never the
  time of the build:
    - the web UI's version identifier (SvelteKit's `version.name`, which
      defaults to the current time) is the commit (`DUMBMONIT_BUILD`), or else
      `SOURCE_DATE_EPOCH`;
    - the files embedded in the server carry no modification date
      (`rust-embed` with `deterministic-timestamps`);
    - the PE header of the Windows agent gets a fixed link date
      (`SOURCE_DATE_EPOCH=0` in the agent stage);
    - BuildKit writes the commit date in the image configuration, its history
      and, with `rewrite-timestamp=true`, the file dates of the layers.
- **No machine paths in the binaries.** The builds run in containers with
  fixed paths (`/build`, `/usr/local/cargo`); the macOS build remaps its paths
  with `--remap-path-prefix`.
- **Fixed build profile.** `Cargo.toml` fixes `lto`, one codegen unit, `strip`
  and `panic = "abort"`.

## How to check it yourself

You need Docker with Buildx (BuildKit 0.13 or later) and a checkout of the
commit or tag to check. Pass the same two inputs as the release workflow, the
commit and its date:

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

Build a second time (another machine, another directory, `--no-cache`) and
compare the checksums: they are identical.

To compare with a published release:

- **Agent binaries:** compare with the `.sha256` files attached to the GitHub
  release of the tag.
- **Server binary:** take it out of the published image and compare:

    ```sh
    docker create --name dm ghcr.io/laupernoe/dumbmonit:0.9.1
    docker cp dm:/dumbmonit published-dumbmonit && docker rm dm
    sha256sum published-dumbmonit out/server/dumbmonit
    ```

- **Images:** build the image as an OCI archive and compare its layers with
  the published ones (the published image also carries labels, a build
  provenance and an SBOM, which change its manifest digest but not its
  layers; agent image: same commands with `--target agent-image` and
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

A mismatch is worth an issue: please open one with the commands you ran.

## Limits

- **Published images before 0.9.1** were built without `SOURCE_DATE_EPOCH`:
  their binaries and layers differ from a rebuild. The comparison of a local
  build with a published image has not been run yet: the first release built
  this way is 0.9.1.
- **The macOS agent** is built on a GitHub macOS runner (Apple's SDK may not
  be redistributed, so it cannot be built in a container). It is identical
  between two builds on the same macOS and Xcode, which the release log
  prints (`xcodebuild -version`); another Xcode may give another binary.
- **Time.** The base images are pinned by digest and stay available, but the
  Alpine package index used for `make` (a build tool, not part of the output),
  npm and crates.io are online services: a rebuild years later depends on them
  still serving the locked versions.
- **Without the build arguments**, a local `docker build .` with no
  `DUMBMONIT_BUILD` and no `SOURCE_DATE_EPOCH` gives a web UI version
  identifier taken from the current time, and therefore a different server
  binary each time. Pass both, as above.

# Reproducible builds

This page states what is done to make builds repeatable and what is not
verified. DumbMonit does **not** claim bit-for-bit reproducible builds today.

## What is in place

- **Locked dependencies.** `Cargo.lock` and `web/package-lock.json` are
  committed, and the `Dockerfile` builds with `cargo build --release --locked`
  (and `cargo zigbuild --release --locked` for the agent), so a build never
  resolves a newer crate than the one reviewed.
- **Pinned build environment.** The base images of the `Dockerfile` (Node, Rust
  on Alpine, the cross-compilation image, Alpine and VictoriaMetrics) are
  pinned by version and by SHA-256 digest; the agent toolchain is pinned with
  `RUST_TOOLCHAIN`. Dependabot proposes updates to them.
- **Fixed build profile.** The release profile (`Cargo.toml`) is fixed:
  `lto`, one codegen unit, `strip`, `panic = "abort"`.
- **Provenance.** Release artifacts carry a SLSA build provenance that names
  the exact commit and workflow that produced them (see
  [Verifying a release](../install/verify.md)).

## What is not verified

- No test rebuilds a release twice and compares the results byte for byte.
- `SOURCE_DATE_EPOCH` is not set, so timestamps and build paths may differ
  between two builds of the same commit.
- The image layers depend on the Alpine package index at build time unless the
  package versions are pinned.

## How to check it yourself

Build the same tag twice, on two machines, and compare the binaries:

```sh
docker buildx build --target agent-dist --output type=local,dest=dist/agent .
sha256sum dist/agent/*
```

Compare with the `.sha256` files of the release. A mismatch is not by itself a
sign of tampering given the points above, but we would like to hear about it:
open an issue. Making the build bit-for-bit reproducible is a goal, not a
promise; it is tracked on the [roadmap](roadmap.md).

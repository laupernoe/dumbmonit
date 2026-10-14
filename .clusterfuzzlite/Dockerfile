FROM gcr.io/oss-fuzz-base/base-builder-rust:v1@sha256:3428d0a90b1e7efbe2ba947bd1d0c9a70594986b4b6553804547f1fb24cfc92f

# La nightly livrée par l'image est plus ancienne que la version minimale de
# Rust du projet (édition 2024, MSRV 1.95) : une nightly datée la remplace.
RUN rustup toolchain install nightly-2026-10-01 --profile minimal
ENV RUSTUP_TOOLCHAIN=nightly-2026-10-01

COPY . $SRC/dumbmonit
WORKDIR $SRC/dumbmonit
COPY .clusterfuzzlite/build.sh $SRC/

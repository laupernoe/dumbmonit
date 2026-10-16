# Verifying a release

Since 0.9.0, everything a release publishes is signed or attested by the
repository's release workflow (`.github/workflows/release.yml`), with
[Sigstore](https://www.sigstore.dev/) keyless signing. There is **no
long-lived signing key** to obtain, protect or rotate: the signing identity is
the GitHub Actions workflow itself, recorded in a short-lived certificate and
in the public Sigstore transparency log. Verifying means checking that the
artifact was built by that workflow, from a `v*` tag of
`laupernoe/dumbmonit`.

## Agent binaries

Each binary on the [GitHub release](https://github.com/laupernoe/dumbmonit/releases/latest)
comes with `<file>.sigstore.json` (the signature bundle) and `<file>.sha256`,
and the release carries `dumbmonit-agent.intoto.jsonl`, the SLSA build
provenance.

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/release\.yml@refs/tags/v'

# or, with the GitHub CLI (checks the provenance attestation)
gh attestation verify dumbmonit-agent-linux-x86_64 -R laupernoe/dumbmonit
```

A successful run prints the workflow identity and the tag. If it fails, do not
install the file, and [report it](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## Container images

The server and agent images carry a build provenance attestation pushed to
ghcr.io, and an SPDX SBOM and full SLSA provenance attached to the image index.

```sh
gh attestation verify oci://ghcr.io/laupernoe/dumbmonit:0.9.0 -R laupernoe/dumbmonit

# the SBOM and the provenance recorded by the build
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .SBOM }}'
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .Provenance }}'
```

Pin an image by digest in your Compose file if you want what you verified to
be exactly what runs.

## Version tags

Release tags (`v0.9.0`, …) are annotated, not GPG-signed, and only repository
administrators can create them (a ruleset reserves `v*`). Trust in a release
rests on the signed artifacts above, whose certificate names the tag they were
built from, rather than on the tag signature.

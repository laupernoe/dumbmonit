# Ein Release überprüfen {#verifying-a-release}

Seit 0.9.0 wird alles, was ein Release veröffentlicht, vom Release-Workflow des
Repositorys (`.github/workflows/release.yml`) mit keyless-Signierung über
[Sigstore](https://www.sigstore.dev/) signiert oder attestiert. Es gibt **keinen
langlebigen Signierschlüssel**, den man beschaffen, schützen oder rotieren müsste:
Die Signaturidentität ist der GitHub-Actions-Workflow selbst, festgehalten in einem
kurzlebigen Zertifikat und im öffentlichen Sigstore-Transparenzlog. Überprüfen heißt
prüfen, dass das Artefakt von diesem Workflow gebaut wurde, aus einem `v*`-Tag von
`laupernoe/dumbmonit`.

## Agent-Binaries {#agent-binaries}

Jedes Binary auf dem [GitHub-Release](https://github.com/laupernoe/dumbmonit/releases/latest)
kommt mit `<file>.sigstore.json` (dem Signaturbündel) und `<file>.sha256`, und das
Release trägt `dumbmonit-agent.intoto.jsonl`, die SLSA-Build-Provenance.

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/release\.yml@refs/tags/v'

# or, with the GitHub CLI (checks the provenance attestation)
gh attestation verify dumbmonit-agent-linux-x86_64 -R laupernoe/dumbmonit
```

Ein erfolgreicher Lauf gibt die Workflow-Identität und den Tag aus. Schlägt er
fehl, installieren Sie die Datei nicht und [melden Sie es](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## Container-Images {#container-images}

Das Server- und das Agent-Image tragen eine nach ghcr.io gepushte
Build-Provenance-Attestierung sowie eine SPDX-SBOM und eine vollständige
SLSA-Provenance am Image-Index.

```sh
gh attestation verify oci://ghcr.io/laupernoe/dumbmonit:0.9.0 -R laupernoe/dumbmonit

# the SBOM and the provenance recorded by the build
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .SBOM }}'
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .Provenance }}'
```

Fixieren Sie ein Image in Ihrer Compose-Datei per Digest, wenn das, was Sie
überprüft haben, genau das sein soll, was läuft.

## Versions-Tags {#version-tags}

Release-Tags (`v0.9.0`, …) sind annotiert, nicht GPG-signiert, und nur
Repository-Administratoren können sie erstellen (ein Ruleset reserviert `v*`). Das
Vertrauen in ein Release beruht auf den signierten Artefakten oben, deren Zertifikat
den Tag nennt, aus dem sie gebaut wurden, und nicht auf der Tag-Signatur.

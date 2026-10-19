# Vérifier une version

Depuis la 0.9.0, tout ce qu'une version publie est signé ou attesté par le
workflow de publication du dépôt (`.github/workflows/release.yml`), avec la
signature sans clé de [Sigstore](https://www.sigstore.dev/). Il n'y a **aucune
clé de signature de longue durée** à obtenir, protéger ou renouveler : l'identité
signataire est le workflow GitHub Actions lui-même, consignée dans un
certificat de courte durée et dans le journal de transparence public de
Sigstore. Vérifier, c'est contrôler que l'artefact a été construit par ce
workflow, à partir d'un tag `v*` de `laupernoe/dumbmonit`.

## Binaires de l'agent

Chaque binaire de la [version GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
est accompagné de `<fichier>.sigstore.json` (le bundle de signature) et de
`<fichier>.sha256`, et la version contient `dumbmonit-agent.intoto.jsonl`, la
provenance de build SLSA.

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/release\.yml@refs/tags/v'

# or, with the GitHub CLI (checks the provenance attestation)
gh attestation verify dumbmonit-agent-linux-x86_64 -R laupernoe/dumbmonit

# or against the provenance attached to the release
gh attestation verify dumbmonit-agent-linux-x86_64 -R laupernoe/dumbmonit \
  --bundle dumbmonit-agent.intoto.jsonl
```

Une exécution réussie affiche l'identité du workflow et le tag. En cas
d'échec, n'installez pas le fichier et
[signalez-le](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## Images de conteneur

Les images du serveur et de l'agent portent une attestation de provenance de
build poussée sur ghcr.io, ainsi qu'un SBOM SPDX et une provenance SLSA complète
attachés à l'index de l'image.

```sh
gh attestation verify oci://ghcr.io/laupernoe/dumbmonit:0.9.0 -R laupernoe/dumbmonit

# the SBOM and the provenance recorded by the build
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .SBOM }}'
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .Provenance }}'
```

Épinglez une image par son digest dans votre fichier Compose si vous voulez que
ce que vous avez vérifié soit exactement ce qui s'exécute.

## Signés après coup {#signed-afterwards}

Les versions antérieures à 0.9.0 (`v0.1.0-alpha.N`), et les fichiers que le workflow de publication n'a pas signés lui-même (comme `provenance.json`), reçoivent leur `<fichier>.sigstore.json` d'un workflow distinct, lancé à la main (`.github/workflows/sign-releases.yml`). Sa signature dit seulement que le fichier était sur la version quand ce workflow a tourné, pas qu'il a été construit depuis le tag : vérifiez-la avec l'identité de ce workflow, et comparez le fichier au `.sha256` qui l'accompagne.

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/sign-releases\.yml@refs/heads/(main|dev)$'
```

Ces versions n'ont pas de provenance de build : `gh attestation verify` ne s'y applique pas.

## Tags de version

Les tags de version (`v0.9.0`, …) sont annotés, pas signés en GPG, et seuls les
administrateurs du dépôt peuvent les créer (une règle réserve `v*`). La
confiance dans une version repose sur les artefacts signés ci-dessus, dont le
certificat nomme le tag à partir duquel ils ont été construits, plutôt que sur
la signature du tag.

# Verificare una release

Dalla 0.9.0, tutto ciò che una release pubblica è firmato o attestato dal
workflow di release del repository (`.github/workflows/release.yml`), con la
firma keyless di [Sigstore](https://www.sigstore.dev/). **Non c'è alcuna
chiave di firma di lunga durata** da ottenere, proteggere o ruotare: l'identità
di firma è il workflow GitHub Actions stesso, registrata in un certificato di
breve durata e nel registro di trasparenza pubblico di Sigstore. Verificare
significa controllare che l'artefatto sia stato compilato da quel workflow, a
partire da un tag `v*` di `laupernoe/dumbmonit`.

## Binari dell'agente

Ogni binario sulla [release GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
è accompagnato da `<file>.sigstore.json` (il bundle della firma) e
`<file>.sha256`, e la release include `dumbmonit-agent.intoto.jsonl`, la
provenienza di build SLSA.

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/release\.yml@refs/tags/v'

# or, with the GitHub CLI (checks the provenance attestation)
gh attestation verify dumbmonit-agent-linux-x86_64 -R laupernoe/dumbmonit
```

Un'esecuzione riuscita stampa l'identità del workflow e il tag. Se fallisce,
non installare il file e [segnalalo](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## Immagini container

Le immagini del server e dell'agente portano un'attestazione di provenienza
di build inviata a ghcr.io, e un SBOM SPDX e una provenienza SLSA completa
allegati all'indice dell'immagine.

```sh
gh attestation verify oci://ghcr.io/laupernoe/dumbmonit:0.9.0 -R laupernoe/dumbmonit

# the SBOM and the provenance recorded by the build
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .SBOM }}'
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .Provenance }}'
```

Fissa un'immagine tramite digest nel tuo file Compose se vuoi che ciò che hai
verificato sia esattamente ciò che gira.

## Tag di versione

I tag di release (`v0.9.0`, …) sono annotati, non firmati con GPG, e solo gli
amministratori del repository possono crearli (un ruleset riserva `v*`). La
fiducia in una release poggia sugli artefatti firmati qui sopra, il cui
certificato nomina il tag da cui sono stati compilati, più che sulla firma del
tag.

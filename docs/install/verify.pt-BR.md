# Verificando uma versão {#verifying-a-release}

Desde a 0.9.0, tudo o que uma versão publica é assinado ou atestado pelo
workflow de release do repositório (`.github/workflows/release.yml`), com
assinatura sem chave do [Sigstore](https://www.sigstore.dev/). **Não há chave
de assinatura de longa duração** para obter, proteger ou rotacionar: a
identidade de assinatura é o próprio workflow do GitHub Actions, registrada em
um certificado de curta duração e no log público de transparência do Sigstore.
Verificar significa conferir que o artefato foi construído por esse workflow, a
partir de uma tag `v*` de `laupernoe/dumbmonit`.

## Binários do agente {#agent-binaries}

Cada binário na [release do GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
acompanha `<arquivo>.sigstore.json` (o pacote de assinatura) e
`<arquivo>.sha256`, e a release traz `dumbmonit-agent.intoto.jsonl`, a
proveniência de build SLSA.

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/release\.yml@refs/tags/v'

# or, with the GitHub CLI (checks the provenance attestation)
gh attestation verify dumbmonit-agent-linux-x86_64 -R laupernoe/dumbmonit
```

Uma execução bem-sucedida imprime a identidade do workflow e a tag. Se falhar,
não instale o arquivo e [relate o problema](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## Imagens de contêiner {#container-images}

As imagens do servidor e do agente trazem uma atestação de proveniência de
build enviada ao ghcr.io, e um SBOM SPDX e a proveniência SLSA completa
anexados ao índice da imagem.

```sh
gh attestation verify oci://ghcr.io/laupernoe/dumbmonit:0.9.0 -R laupernoe/dumbmonit

# the SBOM and the provenance recorded by the build
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .SBOM }}'
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .Provenance }}'
```

Fixe uma imagem pelo digest no seu arquivo Compose se quiser que o que você
verificou seja exatamente o que roda.

## Tags de versão {#version-tags}

As tags de versão (`v0.9.0`, …) são anotadas, não assinadas com GPG, e só os
administradores do repositório podem criá-las (um ruleset reserva `v*`). A
confiança em uma versão se apoia nos artefatos assinados acima, cujo
certificado nomeia a tag a partir da qual foram construídos, e não na
assinatura da tag.

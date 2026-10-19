# Verificar uma versão {#verifying-a-release}

Desde a 0.9.0, tudo o que uma versão publica é assinado ou atestado pelo
fluxo de trabalho de versões do repositório (`.github/workflows/release.yml`),
com assinatura sem chave [Sigstore](https://www.sigstore.dev/). **Não há
nenhuma chave de assinatura de longa duração** para obter, proteger ou rodar:
a identidade de assinatura é o próprio fluxo de trabalho do GitHub Actions,
registada num certificado de curta duração e no registo público de
transparência do Sigstore. Verificar significa confirmar que o artefacto foi
compilado por esse fluxo de trabalho, a partir de uma etiqueta `v*` de
`laupernoe/dumbmonit`.

## Binários do agente {#agent-binaries}

Cada binário da [versão no GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
vem com `<file>.sigstore.json` (o pacote de assinatura) e `<file>.sha256`, e a
versão inclui `dumbmonit-agent.intoto.jsonl`, a proveniência de compilação
SLSA.

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

Uma execução bem-sucedida imprime a identidade do fluxo de trabalho e a
etiqueta. Se falhar, não instale o ficheiro e [reporte-o](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## Imagens de contentor {#container-images}

As imagens do servidor e do agente levam uma atestação de proveniência de
compilação enviada para o ghcr.io, e um SBOM SPDX e a proveniência SLSA
completa anexados ao índice da imagem.

```sh
gh attestation verify oci://ghcr.io/laupernoe/dumbmonit:0.9.0 -R laupernoe/dumbmonit

# the SBOM and the provenance recorded by the build
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .SBOM }}'
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .Provenance }}'
```

Fixe uma imagem pelo digest no seu ficheiro Compose se quiser que o que
verificou seja exatamente o que corre.

## Assinados posteriormente {#signed-afterwards}

As versões anteriores à 0.9.0 (`v0.1.0-alpha.N`), e os ficheiros que o workflow de publicação não assinou ele próprio (como `provenance.json`), recebem o seu `<ficheiro>.sigstore.json` de um workflow separado, executado manualmente (`.github/workflows/sign-releases.yml`). A sua assinatura diz apenas que o ficheiro estava na versão quando esse workflow correu, não que foi compilado a partir da tag: verifique-a com a identidade desse workflow e compare o ficheiro com o `.sha256` que o acompanha.

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/sign-releases\.yml@refs/heads/(main|dev)$'
```

Estas versões não têm proveniência de compilação: `gh attestation verify` não se aplica a elas.

## Etiquetas de versão {#version-tags}

As etiquetas de versão (`v0.9.0`, …) são anotadas, não assinadas com GPG, e só
os administradores do repositório as podem criar (um conjunto de regras
reserva `v*`). A confiança numa versão assenta nos artefactos assinados acima,
cujo certificado nomeia a etiqueta a partir da qual foram compilados, e não na
assinatura da etiqueta.

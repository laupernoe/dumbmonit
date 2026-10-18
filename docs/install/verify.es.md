# Verificar una versión {#verifying-a-release}

Desde la 0.9.0, todo lo que publica una versión está firmado o atestado por el
flujo de trabajo de versiones del repositorio (`.github/workflows/release.yml`),
con la firma sin clave de [Sigstore](https://www.sigstore.dev/). **No hay
ninguna clave de firma de larga duración** que obtener, proteger o rotar: la
identidad firmante es el propio flujo de trabajo de GitHub Actions, registrado
en un certificado de corta duración y en el registro público de transparencia de
Sigstore. Verificar significa comprobar que el artefacto fue compilado por ese
flujo de trabajo, a partir de una etiqueta `v*` de `laupernoe/dumbmonit`.

## Binarios del agente {#agent-binaries}

Cada binario de la [versión de GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
viene con `<file>.sigstore.json` (el paquete de la firma) y `<file>.sha256`, y la
versión incluye `dumbmonit-agent.intoto.jsonl`, la procedencia de compilación
SLSA.

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/release\.yml@refs/tags/v'

# or, with the GitHub CLI (checks the provenance attestation)
gh attestation verify dumbmonit-agent-linux-x86_64 -R laupernoe/dumbmonit
```

Una ejecución correcta imprime la identidad del flujo de trabajo y la etiqueta.
Si falla, no instales el archivo y [infórmalo](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## Imágenes de contenedor {#container-images}

Las imágenes del servidor y del agente llevan una atestación de procedencia de
compilación publicada en ghcr.io, y un SBOM SPDX y la procedencia SLSA completa
adjuntos al índice de la imagen.

```sh
gh attestation verify oci://ghcr.io/laupernoe/dumbmonit:0.9.0 -R laupernoe/dumbmonit

# the SBOM and the provenance recorded by the build
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .SBOM }}'
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .Provenance }}'
```

Fija una imagen por su digest en tu archivo Compose si quieres que lo que
verificaste sea exactamente lo que se ejecuta.

## Etiquetas de versión {#version-tags}

Las etiquetas de versión (`v0.9.0`, …) son anotadas, no están firmadas con GPG, y
solo los administradores del repositorio pueden crearlas (un conjunto de reglas
reserva `v*`). La confianza en una versión descansa en los artefactos firmados
de arriba, cuyo certificado nombra la etiqueta a partir de la cual se
compilaron, y no en la firma de la etiqueta.

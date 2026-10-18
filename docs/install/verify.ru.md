# Проверка релиза

Начиная с 0.9.0 всё, что публикует релиз, подписано или заверено рабочим
процессом релиза репозитория (`.github/workflows/release.yml`) с помощью
бесключевой подписи [Sigstore](https://www.sigstore.dev/). Нет **долгоживущего
ключа подписи**, который нужно получать, защищать или менять: подписывающая
личность — сам рабочий процесс GitHub Actions, записанный в краткосрочном
сертификате и в публичном журнале прозрачности Sigstore. Проверка означает
убедиться, что артефакт собран этим рабочим процессом из тега `v*` репозитория
`laupernoe/dumbmonit`.

## Бинарные файлы агента

К каждому бинарному файлу в [релизе на GitHub](https://github.com/laupernoe/dumbmonit/releases/latest)
прилагаются `<файл>.sigstore.json` (пакет подписи) и `<файл>.sha256`, а сам
релиз содержит `dumbmonit-agent.intoto.jsonl`, происхождение сборки SLSA.

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/release\.yml@refs/tags/v'

# or, with the GitHub CLI (checks the provenance attestation)
gh attestation verify dumbmonit-agent-linux-x86_64 -R laupernoe/dumbmonit
```

Успешный запуск выводит личность рабочего процесса и тег. Если проверка
не проходит, не устанавливайте файл и [сообщите об этом](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## Образы контейнеров

Образы сервера и агента несут аттестацию происхождения сборки, отправленную в
ghcr.io, а также SBOM в формате SPDX и полное происхождение SLSA, прикреплённые
к индексу образа.

```sh
gh attestation verify oci://ghcr.io/laupernoe/dumbmonit:0.9.0 -R laupernoe/dumbmonit

# the SBOM and the provenance recorded by the build
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .SBOM }}'
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .Provenance }}'
```

Закрепите образ по дайджесту в файле Compose, если хотите, чтобы запускалось
именно то, что вы проверили.

## Теги версий

Теги релизов (`v0.9.0`, …) аннотированные, без подписи GPG, и создавать их могут
только администраторы репозитория (набор правил резервирует `v*`). Доверие к
релизу основано на подписанных артефактах выше, сертификат которых называет тег,
из которого они собраны, а не на подписи тега.

# 验证发布版本 {#verifying-a-release}

自 0.9.0 起,发布所公布的一切内容都由仓库的发布工作流
(`.github/workflows/release.yml`)使用 [Sigstore](https://www.sigstore.dev/)
无密钥签名进行签名或证明。**没有任何长期有效的签名密钥**需要获取、
保护或轮换:签名身份就是 GitHub Actions 工作流本身,记录在一份短期
证书和公开的 Sigstore 透明日志中。验证就是检查该产物是由这个工作流、
基于 `laupernoe/dumbmonit` 的某个 `v*` 标签构建的。

## agent 二进制文件 {#agent-binaries}

[GitHub 发布页](https://github.com/laupernoe/dumbmonit/releases/latest)
上的每个二进制文件都附带 `<file>.sigstore.json`(签名包)和
`<file>.sha256`,发布页还附带 `dumbmonit-agent.intoto.jsonl`,即 SLSA
构建来源证明。

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

成功运行会打印工作流身份和标签。如果失败,请不要安装该文件,并
[报告](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md)。

## 容器镜像 {#container-images}

服务器镜像和 agent 镜像带有推送到 ghcr.io 的构建来源证明,以及附加在
镜像索引上的 SPDX SBOM 和完整的 SLSA 来源证明。

```sh
gh attestation verify oci://ghcr.io/laupernoe/dumbmonit:0.9.0 -R laupernoe/dumbmonit

# the SBOM and the provenance recorded by the build
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .SBOM }}'
docker buildx imagetools inspect ghcr.io/laupernoe/dumbmonit:0.9.0 \
  --format '{{ json .Provenance }}'
```

如果你希望所运行的内容恰好就是你所验证的内容,请在 Compose 文件中按
摘要固定镜像。

## 事后签名 {#signed-afterwards}

0.9.0 之前的版本（`v0.1.0-alpha.N`），以及发布工作流本身未签名的文件（例如 `provenance.json`），其 `<文件>.sigstore.json` 来自一个单独、手动运行的工作流（`.github/workflows/sign-releases.yml`）。该签名只表明该工作流运行时文件已在发布中，并不表明它是从该标签构建的：请用该工作流的身份进行验证，并将文件与旁边的 `.sha256` 比对。

```sh
cosign verify-blob dumbmonit-agent-linux-x86_64 \
  --bundle dumbmonit-agent-linux-x86_64.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/laupernoe/dumbmonit/\.github/workflows/sign-releases\.yml@refs/heads/(main|dev)$'
```

这些版本没有构建来源证明：`gh attestation verify` 不适用于它们。

## 版本标签 {#version-tags}

发布标签(`v0.9.0` 等)是带注释的标签,没有 GPG 签名,并且只有仓库
管理员才能创建它们(有一条规则集保留了 `v*`)。对一个发布版本的信任
建立在上述已签名的产物之上,其证书会指明构建所基于的标签,而不是
建立在标签签名上。

#!/bin/sh
# Builds the :edge images on this machine and pushes them to ghcr.io.
#
# A local build reuses its whole layer cache and every core, where a hosted
# runner starts cold on four.
#
# Once: `gh auth refresh -h github.com -s write:packages`, then
#   gh auth token | docker login ghcr.io -u <github user> --password-stdin
set -eu
cd "$(dirname "$0")/.."

if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
	echo "Uncommitted changes: commit them first, :edge must match a commit." >&2
	exit 1
fi

sha=$(git rev-parse HEAD)
source=https://github.com/laupernoe/dumbmonit
labels="--label org.opencontainers.image.source=$source --label org.opencontainers.image.revision=$sha --label org.opencontainers.image.licenses=Apache-2.0"

# shellcheck disable=SC2086 # $labels holds several arguments on purpose.
docker build --platform linux/amd64 --build-arg DUMBMONIT_BUILD="$sha" $labels \
	-t ghcr.io/laupernoe/dumbmonit:edge .
# Same Dockerfile, second target: every layer above comes from the cache.
# shellcheck disable=SC2086
docker build --platform linux/amd64 --build-arg DUMBMONIT_BUILD="$sha" $labels \
	--target agent-image -t ghcr.io/laupernoe/dumbmonit-agent:edge .

docker push ghcr.io/laupernoe/dumbmonit:edge
docker push ghcr.io/laupernoe/dumbmonit-agent:edge
echo "Pushed :edge at $(git rev-parse --short HEAD)."

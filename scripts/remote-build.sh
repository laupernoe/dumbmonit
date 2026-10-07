#!/usr/bin/env bash
# Lance fmt, clippy et les tests Rust sur une machine distante (Docker requis là-bas).
#   REMOTE_BUILD_HOST=admin2@192.168.10.250 scripts/remote-build.sh [fmt|clippy|test|all] [args cargo test]
#   REMOTE_BUILD_KEY=~/.ssh/id_ed25519_forgejo   (optionnel : clé SSH à utiliser)
#   REMOTE_BUILD_DIR=dumbmonit                   (optionnel : dossier distant)
set -euo pipefail

host="${REMOTE_BUILD_HOST:?définir REMOTE_BUILD_HOST, par ex. user@192.168.1.50}"
dir="${REMOTE_BUILD_DIR:-dumbmonit}"
step="${1:-all}"
shift || true

ssh_opts=(-o BatchMode=yes -o ConnectTimeout=10)
if [ -n "${REMOTE_BUILD_KEY:-}" ]; then
  ssh_opts+=(-i "${REMOTE_BUILD_KEY/#\~/$HOME}" -o IdentitiesOnly=yes)
fi

cd "$(dirname "$0")/.."

echo "==> envoi des sources vers $host:$dir"
ssh_cmd="ssh ${ssh_opts[*]}"
rsync -az --delete -e "$ssh_cmd" \
  --exclude=/target --exclude=/.git --exclude=/.claude --exclude=/.local \
  --exclude=node_modules --exclude=.svelte-kit --exclude=/dist --exclude=/website \
  ./ "$host:$dir/"

case "$step" in
  fmt) cmd="cargo fmt --all --check" ;;
  clippy) cmd="cargo clippy --all-targets --all-features -- -D warnings" ;;
  test) cmd="cargo test $*" ;;
  all) cmd="cargo fmt --all --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test $*" ;;
  *) echo "usage: $0 [fmt|clippy|test|all] [args cargo test]" >&2; exit 2 ;;
esac

echo "==> $step sur $host"
ssh "${ssh_opts[@]}" "$host" "cd '$dir' && \
  (docker image inspect dumbmonit-devenv >/dev/null 2>&1 || docker build -t dumbmonit-devenv --target builder .) && \
  docker run --rm -e CARGO_INCREMENTAL=0 -v \"\$PWD:/build\" -w /build \
    -v dumbmonit-cargo:/usr/local/cargo/registry -v dumbmonit-target:/build/target \
    dumbmonit-devenv sh -c 'rustup component add rustfmt clippy >/dev/null 2>&1; $cmd'"

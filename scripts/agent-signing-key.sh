#!/usr/bin/env bash
# Crée la clé Ed25519 qui signe les binaires de l'agent (voir
# docs/development/agent-signing.md). Pour les mainteneurs, une fois, sur une
# machine de confiance — jamais en CI.
#
#   scripts/agent-signing-key.sh [DOSSIER]   (défaut : ~/.dumbmonit-agent-signing,
#                                            hors du dépôt pour ne jamais la committer)
#
# Ce que le script fait :
#   - crée DOSSIER en 0700, la clé privée agent-signing.pem en 0600 ;
#   - écrit la moitié publique dans agent-signing.pub.pem ;
#   - vérifie la paire par une signature d'essai ;
#   - affiche les deux commandes qui restent à lancer, à la main.
#
# Ce qu'il ne fait PAS : toucher au dépôt (release-keys.pem), aux secrets
# GitHub, ni envoyer la clé où que ce soit. Il refuse d'écraser une clé
# existante.
#
# Prérequis : OpenSSL 1.1.1 ou plus récent (Ed25519, `pkeyutl -rawin`).
set -euo pipefail

DIR=${1:-$HOME/.dumbmonit-agent-signing}
KEY="$DIR/agent-signing.pem"
PUB="$DIR/agent-signing.pub.pem"

command -v openssl >/dev/null 2>&1 || { echo "openssl is required" >&2; exit 1; }
if [ -e "$KEY" ]; then
    echo "Refusing to overwrite $KEY: move it away first." >&2
    exit 1
fi

umask 077
mkdir -p "$DIR"
chmod 700 "$DIR"

openssl genpkey -algorithm ed25519 -out "$KEY"
chmod 600 "$KEY"
openssl pkey -in "$KEY" -pubout -out "$PUB"

# Signature d'essai, exactement comme release.yml la produit et comme l'agent
# la vérifie (signature détachée de 64 octets bruts).
probe=$(mktemp)
trap 'rm -f "$probe" "$probe.sig"' EXIT
head -c 4096 /dev/urandom > "$probe"
openssl pkeyutl -sign -rawin -inkey "$KEY" -in "$probe" -out "$probe.sig"
[ "$(wc -c < "$probe.sig" | tr -d ' ')" = 64 ] || { echo "unexpected signature size" >&2; exit 1; }
openssl pkeyutl -verify -rawin -pubin -inkey "$PUB" -in "$probe" -sigfile "$probe.sig" >/dev/null

cat <<FIN
Ed25519 agent signing key created:
  private key: $KEY  (keep it offline; never commit it)
  public key:  $PUB

Public key, to append to crates/agent/signing/release-keys.pem (commit it):

$(cat "$PUB")

Then store the private key as the repository secret used by release.yml:

  gh secret set AGENT_SIGNING_KEY < "$KEY"

Keep an offline copy of $KEY, then delete it from this machine.
FIN

#!/bin/sh
# Test de charge de DumbMonit : démarre l'image sur un volume neuf, crée N
# surveillances de démonstration (collecteur `dummy`, sans réseau), laisse tourner,
# mesure CPU, mémoire, disque et latence de l'API, écrit summary.json et summary.md.
#
#   MONITORS=1000 DURATION_MIN=30 scripts/load/run.sh [répertoire-de-sortie]
#
# Variables : MONITORS (1000), DURATION_MIN (30), DUMBMONIT_IMAGE
# (ghcr.io/laupernoe/dumbmonit:edge), DUMBMONIT_LOAD_PORT (18080),
# SAMPLE_SECONDS (60), KEEP=1 pour garder l'instance et son volume à la fin.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
OUT=${1:-"$HERE/out"}
MONITORS=${MONITORS:-1000}
DURATION_MIN=${DURATION_MIN:-30}
export DUMBMONIT_IMAGE=${DUMBMONIT_IMAGE:-ghcr.io/laupernoe/dumbmonit:edge}
export DUMBMONIT_LOAD_PORT=${DUMBMONIT_LOAD_PORT:-18080}
export DUMBMONIT_LOAD_SETUP_CODE=${DUMBMONIT_LOAD_SETUP_CODE:-LOADT-EST42}
ALPINE='alpine:3.24@sha256:be2656061b536c1883c4034790e21d0a09c5aaaf51968b89368a3c0371ea2c9a'
COMPOSE="docker compose -f $HERE/docker-compose.load.yml"

mkdir -p "$OUT"
cleanup() {
    if [ "${KEEP:-0}" != 1 ]; then
        $COMPOSE down --volumes --timeout 30 >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT INT TERM

# Départ propre : un volume restant d'un essai précédent fausserait le disque.
$COMPOSE down --volumes --timeout 30 >/dev/null 2>&1 || true
docker pull --quiet "$ALPINE" >/dev/null
$COMPOSE up -d --pull always

python3 -I "$HERE/load.py" \
    --base "http://127.0.0.1:$DUMBMONIT_LOAD_PORT" \
    --setup-code "$DUMBMONIT_LOAD_SETUP_CODE" \
    --container "$($COMPOSE ps -q dumbmonit)" \
    --volume dumbmonit-load-data \
    --alpine "$ALPINE" \
    --image "$DUMBMONIT_IMAGE" \
    --monitors "$MONITORS" \
    --duration-min "$DURATION_MIN" \
    --sample-seconds "${SAMPLE_SECONDS:-60}" \
    --out "$OUT"

echo "Résumé : $OUT/summary.md et $OUT/summary.json"

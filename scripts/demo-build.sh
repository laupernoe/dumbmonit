#!/usr/bin/env bash
# Builds the static demo (https://dumbmonit.selfhostr.io/demo/): the web UI alone,
# answering from recorded JSON, no backend.
#
#   1. Start a server in demo mode (DUMBMONIT_DEMO=1) and record it:
#        docker run --rm --network host -v "$PWD":/work -w /work \
#          -e NODE_PATH=/usr/src/app/node_modules --entrypoint node \
#          zenika/alpine-chrome:with-puppeteer \
#          scripts/demo-record.mjs http://127.0.0.1:8080 --out /work/demo-record
#   2. Build:  scripts/demo-build.sh demo-record demo-dist
#   3. Publish the content of demo-dist/ to Cloudflare Pages (it already holds /demo/).
#
# Build-time switches (normal builds set none of them and are unchanged):
#   VITE_STATIC_DEMO=1          routes every API call to the recorded answers
#   DUMBMONIT_BASE_PATH=/demo   SvelteKit `paths.base`, and `VITE_BASE_PATH` for the links
#
# Output:  <out>/demo/...         the app, `index.html` being the SPA fallback
#          <out>/demo/demo-data/  index.json + f/*.json
#          <out>/_redirects       the rule sending deep links (/demo/targets/3) to the fallback;
#                                 MERGE it into the site's own _redirects if one exists.
set -euo pipefail

record_dir=${1:?usage: demo-build.sh <recorded-dir> [out-dir]}
out_dir=${2:-demo-dist}
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

[ -f "$record_dir/index.json" ] || { echo "no index.json in $record_dir: run scripts/demo-record.mjs first" >&2; exit 1; }
record_dir=$(cd "$record_dir" && pwd)
mkdir -p "$out_dir"
out_dir=$(cd "$out_dir" && pwd)

cd "$root/web"
[ -d node_modules ] || npm ci
rm -rf build
VITE_STATIC_DEMO=1 DUMBMONIT_BASE_PATH=/demo npm run build

rm -rf "$out_dir/demo"
mkdir -p "$out_dir/demo"
cp -R build/. "$out_dir/demo/"
mkdir -p "$out_dir/demo/demo-data"
cp -R "$record_dir/." "$out_dir/demo/demo-data/"

# Static files win over the rule on Cloudflare Pages; everything else under /demo/ is the app.
printf '/demo/*  /demo/index.html  200\n' > "$out_dir/_redirects"

echo "static demo ready in $out_dir (app at /demo/, $(du -sk "$out_dir/demo/demo-data" | cut -f1) KiB of recorded data)"

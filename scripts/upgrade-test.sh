#!/usr/bin/env bash
# Test de mise à jour : l'image publiée (OLD) crée des données sur un volume, puis
# l'image candidate (NEW) reprend le MÊME volume. Vérifie que rien n'est perdu.
#
#   scripts/upgrade-test.sh [NEW_IMAGE]
#
# Variables :
#   OLD_IMAGE   image de départ (défaut : dernière release publiée, voir plus bas)
#   NEW_IMAGE   image candidate (défaut : ghcr.io/laupernoe/dumbmonit:edge)
#   KEEP=1      garde le volume et les conteneurs après coup (débogage)
#
# Prérequis : docker, curl, python3. Les images sont lancées comme en production
# (système de fichiers en lecture seule, capacités retirées sauf celles qu'il faut
# pour remettre /data à 65532 avant d'abandonner root, comme docker-compose.yml).
set -euo pipefail

REPO=laupernoe/dumbmonit
NEW_IMAGE=${1:-${NEW_IMAGE:-ghcr.io/$REPO:edge}}
PASSWORD='correct horse battery 42'
USER_NAME=upgrader
SETUP_CODE=UPGRD-TEST2
RUN_ID=$$
VOLUME=dumbmonit-upgrade-$RUN_ID
NAME=dumbmonit-upgrade-$RUN_ID
WORK=$(mktemp -d)
JAR=$WORK/cookies
PORT=
CODE_SEEN=

step() { printf '\n==> %s\n' "$*"; }
fail() {
  printf '\nECHEC : %s\n' "$*" >&2
  if docker inspect "$NAME" >/dev/null 2>&1; then
    printf '\n--- 60 dernières lignes du journal de %s ---\n' "$NAME" >&2
    docker logs --tail 60 "$NAME" 2>&1 | sed 's/\x1b\[[0-9;]*m//g' >&2 || true
  fi
  exit 1
}
cleanup() {
  if [ "${KEEP:-0}" = 1 ]; then
    echo "KEEP=1 : volume $VOLUME et dossier $WORK conservés."
    return
  fi
  docker rm -f "$NAME" >/dev/null 2>&1 || true
  docker volume rm "$VOLUME" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

for tool in docker curl python3; do
  command -v "$tool" >/dev/null || { echo "$tool est requis." >&2; exit 2; }
done

# Dernière release publiée : le tag `latest` n'est posé que par un tag v*, donc
# il désigne exactement la dernière release. À défaut (image absente), on prend
# le dernier tag via gh.
resolve_old_image() {
  if [ -n "${OLD_IMAGE:-}" ]; then echo "$OLD_IMAGE"; return; fi
  if docker manifest inspect "ghcr.io/$REPO:latest" >/dev/null 2>&1; then
    echo "ghcr.io/$REPO:latest"; return
  fi
  if command -v gh >/dev/null; then
    local tag
    tag=$(gh api "repos/$REPO/releases/latest" --jq .tag_name 2>/dev/null || true)
    [ -n "$tag" ] && { echo "ghcr.io/$REPO:${tag#v}"; return; }
  fi
  echo "Impossible de trouver la dernière release (fixez OLD_IMAGE)." >&2
  exit 2
}
OLD_IMAGE=$(resolve_old_image)

# --- Aides -------------------------------------------------------------------

# Extrait un champ JSON : `jfield '<expr python sur d>'`, JSON sur stdin.
jfield() { python3 -I -c 'import sys,json; d=json.load(sys.stdin); print(eval(sys.argv[1]))' "$1"; }

# api METHODE CHEMIN [JSON] : imprime le corps, échoue sur un code >= 400.
api() {
  local method=$1 path=$2 body=${3:-} out code
  local args=(-s -o "$WORK/body" -w '%{http_code}' -X "$method" -b "$JAR" -c "$JAR"
    -H 'X-Requested-With: dumbmonit' "http://127.0.0.1:$PORT$path")
  [ -n "$body" ] && args+=(-H 'content-type: application/json' -d "$body")
  code=$(curl "${args[@]}") || fail "curl $method $path n'a pas abouti"
  out=$(cat "$WORK/body")
  if [ "$code" -ge 400 ]; then fail "$method $path -> HTTP $code : $out"; fi
  printf '%s' "$out"
}

start() { # start IMAGE : lance le conteneur sur le volume, attend /api/health
  local image=$1
  docker rm -f "$NAME" >/dev/null 2>&1 || true
  docker run -d --name "$NAME" -p 127.0.0.1::8080 -v "$VOLUME:/data" \
    -e DUMBMONIT_SETUP_CODE="$SETUP_CODE" \
    --read-only --tmpfs /tmp --cap-drop ALL \
    --cap-add CHOWN --cap-add DAC_READ_SEARCH --cap-add SETUID --cap-add SETGID \
    --security-opt no-new-privileges:true \
    --sysctl net.ipv4.ping_group_range="0 2147483647" "$image" >/dev/null \
    || fail "docker run $image"
  PORT=$(docker port "$NAME" 8080/tcp | head -1 | sed 's/.*://')
  local i
  for i in $(seq 1 90); do
    if [ "$(docker inspect -f '{{.State.Running}}' "$NAME")" != true ]; then
      fail "$image s'est arrêtée au démarrage"
    fi
    if curl -fs "http://127.0.0.1:$PORT/api/health" >"$WORK/health" 2>/dev/null; then return; fi
    sleep 1
  done
  fail "$image ne répond pas sur /api/health après 90 s"
}

stop() { docker stop -t 30 "$NAME" >/dev/null || fail "arrêt de $NAME"; }

login() {
  : >"$JAR"
  api POST /api/auth/login "{\"username\":\"$USER_NAME\",\"password\":\"$PASSWORD\"}" >/dev/null
}

# Photographie normalisée de la configuration : seuls les champs stables et
# significatifs sont gardés, pour qu'une nouvelle version puisse en ajouter.
snapshot() {
  python3 -I - "$PORT" "$JAR" >"$1" <<'PY'
import json, subprocess, sys
port, jar = sys.argv[1], sys.argv[2]
def get(path):
    out = subprocess.run(["curl", "-fs", "-b", jar, "-H", "X-Requested-With: dumbmonit",
                          f"http://127.0.0.1:{port}{path}"], capture_output=True, text=True)
    if out.returncode != 0:
        print(f"GET {path} a échoué", file=sys.stderr); sys.exit(1)
    return json.loads(out.stdout)
def pick(items, keys):
    return {str(i["id"]): {k: i.get(k) for k in keys} for i in items}
snap = {
    "users": pick(get("/api/users"), ["username", "role", "disabled"]),
    "targets": pick(get("/api/targets"), ["name", "address", "kind", "tags", "interval_secs",
                                          "enabled", "credential_kind", "group_name"]),
    "rules": pick(get("/api/alerts/rules"), ["name", "query", "threshold", "severity"]),
    "silences": pick(get("/api/alerts/silences"), ["name", "matchers", "schedule", "comment"]),
    "channels": pick(get("/api/notify/channels"), ["name", "kind", "settings", "has_secret"]),
}
pages = get("/api/status-pages")
snap["pages"] = pick(pages, ["slug", "title", "published"])
snap["page_items"] = {str(p["id"]): [[i.get("target_id"), i.get("label")] for i in p.get("items", [])]
                      for p in pages}
json.dump(snap, sys.stdout, indent=1, sort_keys=True)
PY
}

# Échantillons `dumbmonit_up` sur une fenêtre FIXE (millisecondes, bornes passées
# en argument) : la même avant et après, donc comparable. Imprime « nombre plus_ancien ».
history() {
  api GET "/api/metrics/query_range?query=dumbmonit_up&start=$1&end=$2&step=15" \
    | python3 -I -c '
import sys, json
res = json.load(sys.stdin)
pts = [v for r in res for v in r["values"]]
print(len(pts), min((float(v[0]) for v in pts), default=0))'
}

# --- Départ : image publiée --------------------------------------------------

step "Image de départ : $OLD_IMAGE"
step "Image candidate : $NEW_IMAGE"
docker pull -q "$OLD_IMAGE" >/dev/null || fail "pull $OLD_IMAGE"
if ! docker image inspect "$NEW_IMAGE" >/dev/null 2>&1; then
  docker pull -q "$NEW_IMAGE" >/dev/null || fail "pull $NEW_IMAGE"
fi
docker volume create "$VOLUME" >/dev/null

step "Démarrage de l'ancienne version"
start "$OLD_IMAGE"
OLD_VERSION=$(jfield 'd["version"]' <"$WORK/health")
echo "version de départ : $OLD_VERSION"

step "Création du compte et des données"
api POST /api/auth/setup \
  "{\"setup_code\":\"$SETUP_CODE\",\"username\":\"$USER_NAME\",\"password\":\"$PASSWORD\"}" >/dev/null
login

# Appareils de types variés. Ils n'ont pas à être joignables : la configuration,
# les secrets chiffrés et l'historique de `dumbmonit_up` suffisent au test.
T_HTTP=$(api POST /api/targets '{"name":"web self","address":"127.0.0.1:8080","kind":"http","tags":{"scheme":"http","allow_private_targets":"true"},"group_name":"Lab","interval_secs":15}' | jfield 'd["id"]')
T_TCP=$(api POST /api/targets '{"name":"tcp self","address":"127.0.0.1:8080","kind":"tcp","tags":{"allow_private_targets":"true"},"interval_secs":15}' | jfield 'd["id"]')
T_PING=$(api POST /api/targets '{"name":"ping loopback","address":"127.0.0.1","kind":"ping","tags":{"allow_private_targets":"true"},"interval_secs":15}' | jfield 'd["id"]')
T_SNMP=$(api POST /api/targets '{"name":"switch","address":"192.0.2.10","kind":"snmp","interval_secs":60,"credential":{"type":"snmp_community","community":"s3cret-community"}}' | jfield 'd["id"]')
T_PVE=$(api POST /api/targets '{"name":"pve","address":"192.0.2.20","kind":"proxmox","interval_secs":60,"credential":{"type":"api_token","token":"root@pam!dm=00000000-0000-0000-0000-000000000000"}}' | jfield 'd["id"]')
echo "appareils : $T_HTTP $T_TCP $T_PING $T_SNMP $T_PVE"

api POST /api/alerts/rules '{"name":"Upgrade test: target down","query":"dumbmonit_up == 0","operator":">","threshold":0,"for_secs":60,"severity":"warning"}' >/dev/null
api POST /api/alerts/silences '{"name":"Upgrade test silence","comment":"kept","matchers":{"host":"switch"},"schedule":{"kind":"weekly","days":[6],"start_minute":120,"end_minute":240,"utc_offset_minutes":60}}' >/dev/null
CHANNEL_OUT=$(api POST /api/notify/channels '{"name":"Upgrade test ntfy","kind":"ntfy","settings":{"topic":"upgrade-test-topic"}}')
PAGE_ID=$(api POST /api/status-pages '{"title":"Upgrade test","slug":"upgrade-test","published":true}' | jfield 'd["id"]')
api PUT "/api/status-pages/$PAGE_ID/items" "[{\"target_id\":$T_HTTP,\"label\":\"Web\"},{\"target_id\":$T_TCP}]" >/dev/null
echo "règle, silence, canal ($(printf '%s' "$CHANNEL_OUT" | jfield 'd["id"]')) et page $PAGE_ID créés"

step "Attente des premières métriques (jusqu'à 120 s)"
BEFORE_COUNT=0
W_START=$(( ($(date +%s) - 3600) * 1000 ))
for _ in $(seq 1 60); do
  W_END=$(( $(date +%s) * 1000 ))
  read -r BEFORE_COUNT BEFORE_OLDEST < <(history "$W_START" "$W_END" || echo "0 0")
  [ "${BEFORE_COUNT:-0}" -ge 4 ] && break
  sleep 2
done
[ "${BEFORE_COUNT:-0}" -ge 4 ] || fail "moins de 4 échantillons dumbmonit_up après 120 s (obtenus : ${BEFORE_COUNT:-0})"
echo "échantillons avant : $BEFORE_COUNT (plus ancien : $BEFORE_OLDEST)"
api GET /api/status-pages >/dev/null
snapshot "$WORK/before.json"
curl -fs "http://127.0.0.1:$PORT/api/public/status/upgrade-test" >/dev/null \
  || fail "la page publique upgrade-test ne répond pas avant la mise à jour"

step "Arrêt propre de l'ancienne version"
stop

# --- Arrivée : image candidate sur le même volume -----------------------------

step "Démarrage de la version candidate sur le même volume"
start "$NEW_IMAGE"
NEW_VERSION=$(jfield 'd["version"]' <"$WORK/health")
echo "version candidate : $NEW_VERSION ($(jfield 'd.get("build","?")' <"$WORK/health"))"

step "Vérifications"
OK=1
check() { printf '  [%s] %s\n' "$1" "$2"; [ "$1" = ok ] || OK=0; }

LOGS=$(docker logs "$NAME" 2>&1 | sed 's/\x1b\[[0-9;]*m//g')
[ "$(jfield 'd["status"]' <"$WORK/health")" = ok ] && check ok "/api/health : status ok" \
  || check KO "/api/health : $(cat "$WORK/health")"
[ "$(jfield 'd["database"]["ok"]' <"$WORK/health")" = True ] && check ok "base de données saine" \
  || check KO "base de données : $(cat "$WORK/health")"
[ "$(jfield 'd["victoria"]["ok"]' <"$WORK/health")" = True ] && check ok "VictoriaMetrics saine" \
  || check KO "VictoriaMetrics : $(cat "$WORK/health")"
# Les migrations s'appliquent avant « database ready » ; un échec arrête le serveur.
grep -q 'database ready' <<<"$LOGS" && check ok "migrations appliquées (database ready)" \
  || check KO "« database ready » absent du journal"
if grep -Eq ' (ERROR|PANIC) |panicked at' <<<"$LOGS"; then
  check KO "erreurs dans le journal de démarrage :"; grep -E ' (ERROR|PANIC) |panicked at' <<<"$LOGS" | head -5
else
  check ok "aucune ERROR ni panic dans le journal"
fi

if login 2>/dev/null; then check ok "connexion avec le même mot de passe"
else check KO "connexion refusée avec le mot de passe d'avant la mise à jour"; fi

if [ "$OK" = 1 ]; then
  snapshot "$WORK/after.json" || check KO "lecture de la configuration impossible"
  python3 -I - "$WORK/before.json" "$WORK/after.json" <<'PY' || OK=0
import json, sys
before, after = (json.load(open(p)) for p in sys.argv[1:3])
bad = 0
for section, objs in before.items():
    if section == "page_items":
        for pid, items in objs.items():
            ok = after[section].get(pid) == items
            print(f"  [{'ok' if ok else 'KO'}] services de la page {pid}")
            if not ok: bad += 1; print(f"       avant {items}\n       après {after[section].get(pid)}")
        continue
    for oid, fields in objs.items():
        now = after[section].get(oid)
        if now is None:
            print(f"  [KO] {section} #{oid} ({fields.get('name') or fields.get('username') or fields.get('slug')}) a disparu"); bad += 1
        elif now != fields:
            diff = {k: (fields[k], now.get(k)) for k in fields if fields[k] != now.get(k)}
            print(f"  [KO] {section} #{oid} modifié (avant, après) : {diff}"); bad += 1
    print(f"  [{'ok' if not bad else 'KO'}] {section} : {len(objs)} objet(s) inchangé(s)" if not bad else f"  [KO] {section}")
sys.exit(1 if bad else 0)
PY
fi

# Les secrets chiffrés doivent rester lisibles par la nouvelle version : une sonde
# immédiate chiffre/déchiffre côté serveur avec la clé de /data/secret.key.
PROBE=$(curl -s -b "$JAR" -X POST -H 'X-Requested-With: dumbmonit' \
  "http://127.0.0.1:$PORT/api/targets/$T_SNMP/probe" || true)
if grep -qi 'decrypt\|cipher' <<<"$PROBE"; then
  check KO "déchiffrement du secret de l'appareil : $PROBE"
else
  check ok "secrets des appareils toujours déchiffrables"
fi

curl -fs "http://127.0.0.1:$PORT/api/public/status/upgrade-test" >/dev/null \
  && check ok "page de statut publique toujours servie" \
  || check KO "page de statut publique /api/public/status/upgrade-test indisponible"

# L'historique : sur la même fenêtre, les points d'avant sont là, à l'identique.
read -r AFTER_COUNT AFTER_OLDEST < <(history "$W_START" "$W_END" || echo "0 0")
if [ "${AFTER_COUNT:-0}" -eq "$BEFORE_COUNT" ] \
   && python3 -I -c 'import sys; sys.exit(0 if float(sys.argv[1]) == float(sys.argv[2]) else 1)' "$AFTER_OLDEST" "$BEFORE_OLDEST"; then
  check ok "historique VictoriaMetrics conservé ($BEFORE_COUNT -> $AFTER_COUNT échantillons, plus ancien $BEFORE_OLDEST -> $AFTER_OLDEST)"
else
  check KO "historique perdu : $BEFORE_COUNT -> ${AFTER_COUNT:-0} échantillons, plus ancien $BEFORE_OLDEST -> ${AFTER_OLDEST:-?}"
fi

printf '\n'
if [ "$OK" = 1 ]; then
  echo "SUCCES : $OLD_VERSION ($OLD_IMAGE) -> $NEW_VERSION ($NEW_IMAGE), données intactes."
else
  fail "la mise à jour $OLD_VERSION -> $NEW_VERSION a abîmé des données (voir [KO] ci-dessus)."
fi
